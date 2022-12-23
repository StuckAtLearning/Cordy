use std::collections::VecDeque;

use crate::vm::error::RuntimeError;
use crate::vm::value::{Mut, Value};
use crate::vm::VirtualInterface;

use RuntimeError::{*};
use Value::{*};

type ValueResult = Result<Value, Box<RuntimeError>>;


macro_rules! slice_arg {
    ($a:ident, $n:expr, $def:expr) => {
        match $a {
            Value::Int(x) => x,
            Value::Nil => $def,
            t => return TypeErrorSliceArgMustBeInt($n, t).err(),
        }
    };
}


pub fn list_index(list_ref: Mut<VecDeque<Value>>, r: i64) -> ValueResult {
    let list = list_ref.unbox();
    let index: usize = if r < 0 { (list.len() as i64 + r) as usize } else { r as usize };
    if index < list.len() {
        Ok(list[index].clone())
    } else {
        IndexOutOfBounds(r, list.len()).err()
    }
}

pub fn list_slice(a1: Value, a2: Value, a3: Value, a4: Value) -> ValueResult {

    let list_ref= match a1 {
        List(ls) => ls,
        t => return TypeErrorCannotSlice(t).err(),
    };
    let list = list_ref.unbox();
    let length: i64 = list.len() as i64;

    let step: i64 = slice_arg!(a4, "step", 1);
    if step == 0 {
        return SliceStepZero.err()
    }

    let low: i64 = slice_arg!(a2, "low", if step > 0 { 0 } else { -1 });
    let high: i64 = slice_arg!(a3, "high", if step > 0 { 0 } else { -length - 1 });

    let abs_start: i64 = to_index(length, low);
    let abs_step: usize = step.unsigned_abs() as usize;

    return Ok(if step > 0 {
        let abs_stop: i64 = to_index(length, high - 1);

        Value::iter_list((abs_start..=abs_stop).step_by(abs_step)
            .filter_map(|i| safe_get(&list, i))
            .cloned())
    } else {
        let abs_stop: i64 = to_index(length, high);

        Value::iter_list(rev_range(abs_start, abs_stop).step_by(abs_step)
            .filter_map(|i| safe_get(&list, i))
            .cloned())
    })
}


#[inline(always)]
fn to_index(len: i64, pos_or_neg: i64) -> i64 {
    if pos_or_neg >= 0 {
        pos_or_neg
    } else {
        len + pos_or_neg
    }
}

#[inline(always)]
fn safe_get(list: &VecDeque<Value>, index: i64) -> Option<&Value> {
    if index < 0 {
        None
    } else {
        list.get(index as usize)
    }
}

#[inline(always)]
fn rev_range(start_high_inclusive: i64, stop_low_exclusive: i64) -> impl Iterator<Item = i64> {
    let mut start: i64 = start_high_inclusive;
    let end: i64 = stop_low_exclusive;
    std::iter::from_fn(move || {
        if start <= end {
            None
        } else {
            start -= 1;
            Some(start + 1)
        }
    })
}


// ===== Library Functions ===== //

pub fn sum<'a>(args: impl Iterator<Item=&'a Value>) -> ValueResult {
    let mut sum: i64 = 0;
    for v in args {
        match v {
            Int(i) => sum += i,
            _ => return TypeErrorArgMustBeInt(v.clone()).err(),
        }
    }
    Ok(Int(sum))
}

pub fn max<'a>(args: impl Iterator<Item=&'a Value>) -> ValueResult {
    match args.max() {
        Some(v) => Ok(v.clone()),
        None => TypeErrorArgMustNotBeEmpty.err()
    }
}

pub fn min<'a>(args: impl Iterator<Item=&'a Value>) -> ValueResult {
    match args.min() {
        Some(v) => Ok(v.clone()),
        None => TypeErrorArgMustNotBeEmpty.err()
    }
}

pub fn sorted<'a>(args: impl Iterator<Item=&'a Value>) -> ValueResult {
    let mut sorted: Vec<Value> = args.cloned().collect::<Vec<Value>>();
    sorted.sort_unstable();
    Ok(Value::list(sorted))
}

pub fn reversed<'a>(args: impl DoubleEndedIterator<Item=&'a Value>) -> ValueResult {
    Ok(Value::iter_list(args.rev().cloned()))
}


pub fn map<VM>(vm: &mut VM, a1: Value, a2: Value) -> ValueResult where VM : VirtualInterface {
    let len: usize = a2.len().unwrap_or(0);
    match (a1, a2.as_iter()) {
        (l, Ok(rs)) => {
            let rs = (&rs).into_iter();
            let mut acc: Vec<Value> = Vec::with_capacity(len);
            for r in rs {
                vm.push(r.clone());
                vm.push(l.clone());
                let f = match vm.invoke_func_compose() {
                    Err(e) => return e.err(),
                    Ok(f) => f
                };
                vm.run_after_invoke(f)?;
                acc.push(vm.pop());
            }
            Ok(Value::list(acc))
        },
        (_, Err(e)) => return Err(e),
    }
}

pub fn filter<VM>(vm: &mut VM, a1: Value, a2: Value) -> ValueResult where VM : VirtualInterface {
    let len: usize = a2.len().unwrap_or(0);
    match (a1, a2.as_iter()) {
        (l, Ok(rs)) => {
            let rs = (&rs).into_iter();
            let mut acc: Vec<Value> = Vec::with_capacity(len);
            for r in rs {
                vm.push(r.clone());
                vm.push(l.clone());
                let f = match vm.invoke_func_compose() {
                    Err(e) => return e.err(),
                    Ok(f) => f
                };
                vm.run_after_invoke(f)?;
                if vm.pop().as_bool() {
                    acc.push(r.clone());
                }
            }
            Ok(Value::list(acc))
        },
        (_, Err(e)) => Err(e),
    }
}

pub fn reduce<VM>(vm: &mut VM, a1: Value, a2: Value) -> ValueResult where VM : VirtualInterface {
    match (a1, a2.as_iter()) {
        (l, Ok(rs)) => {
            let mut iter = (&rs).into_iter().cloned();
            let mut acc: Value = match iter.next() {
                Some(v) => v,
                None => return TypeErrorArgMustNotBeEmpty.err()
            };

            for r in iter {
                vm.push(l.clone()); // Function
                vm.push(acc); // Accumulator (arg1)
                vm.push(r); // Value (arg2)
                let f = match vm.invoke_func_eval(2) {
                    Err(e) => return e.err(),
                    Ok(f) => f
                };
                vm.run_after_invoke(f)?;
                acc = vm.pop();
            }
            Ok(acc)
        },
        (_, Err(e)) => Err(e),
    }
}

pub fn pop(a1: Value) -> ValueResult {
    match match &a1 {
        List(v) => v.unbox_mut().pop_back(),
        Set(v) => v.unbox_mut().pop_back(),
        Dict(v) => v.unbox_mut().pop_back().map(|(k, _)| k),
        _ => return TypeErrorArgMustBeIterable(a1).err()
    } {
        Some(v) => Ok(v),
        None => TypeErrorArgMustNotBeEmpty.err()
    }
}

pub fn push(a1: Value, a2: Value) -> ValueResult {
    match &a2 {
        List(v) => { v.unbox_mut().push_back(a1); Ok(a2) }
        Set(v) => { v.unbox_mut().insert(a1); Ok(a2) }
        _ => TypeErrorArgMustBeIterable(a2).err()
    }
}

pub fn last(a1: Value) -> ValueResult {
    match &a1 {
        List(v) => Ok(v.unbox_mut().back().cloned().unwrap_or(Nil)),
        Set(v) => Ok(v.unbox_mut().back().cloned().unwrap_or(Nil)),
        _ => TypeErrorArgMustBeIterable(a1).err()
    }
}

pub fn head(a1: Value) -> ValueResult {
    match &a1 {
        List(v) => Ok(v.unbox_mut().front().cloned().unwrap_or(Nil)),
        Set(v) => Ok(v.unbox_mut().front().cloned().unwrap_or(Nil)),
        _ => TypeErrorArgMustBeIterable(a1).err()
    }
}

pub fn init(a1: Value) -> ValueResult {
    match &a1 {
        List(v) => {
            let v = v.unbox();
            let mut iter = v.iter();
            iter.next_back();
            Ok(Value::iter_list(iter.cloned()))
        },
        Set(v) => {
            let v = v.unbox();
            let mut iter = v.iter();
            iter.next_back();
            Ok(Value::iter_set(iter.cloned()))
        },
        _ => TypeErrorArgMustBeIterable(a1).err()
    }
}

pub fn tail(a1: Value) -> ValueResult {
    match &a1 {
        List(v) => Ok(Value::iter_list(v.unbox().iter().skip(1).cloned())),
        Set(v) => Ok(Value::iter_set(v.unbox().iter().skip(1).cloned())),
        _ => TypeErrorArgMustBeIterable(a1).err()
    }
}
