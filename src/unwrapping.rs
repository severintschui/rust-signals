#![allow(dead_code, unused_variables, unused_imports)]

use std::pin::Pin;
use futures_signals::signal::{Mutable, Signal, SignalExt};
use futures_signals::signal_vec::{SignalVec, SignalVecExt};


type AnySignal<T> = Pin<Box<dyn Signal<Item=T> + Send + Sync>>;

struct MyStruct {
    a: Mutable<f64>
}

impl MyStruct {
    fn new(a: f64) -> Self {
        MyStruct {
            a: Mutable::new(a),
        }
    }

    fn a(&self) -> impl Signal<Item=f64> {
        self.a.signal()
    }

    fn a_squared(&self) -> impl Signal<Item=f64> {
        self.a().map(|a| a * a)
    }
}

fn unwrap_signal_field(input: impl Signal<Item=MyStruct>) -> impl Signal<Item=f64> {
    input.switch(|item| item.a_squared())
}

fn unwrap_vec_of_signals(input: impl Signal<Item=Vec<impl Signal<Item=f64>>>) -> impl Signal<Item=Vec<f64>> {
    input.to_signal_vec().map_signal(|item| item).to_signal_cloned()
}

fn flatten_vec<T>(input: Vec<Vec<T>>) -> Vec<T> {
    input.into_iter().flat_map(|vec| vec.into_iter()).collect()
}



mod tests {
    use super::*;

    #[test]
    fn test() {
    }
}
