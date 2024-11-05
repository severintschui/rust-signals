#![allow(dead_code, unused_variables, unused_imports)]

use std::pin::Pin;
use std::sync::{Arc, OnceLock, Weak};
use futures_signals::signal::{Broadcaster, Mutable, Signal, SignalExt};
use futures_signals::signal_vec::{MutableVec, SignalVec, SignalVecExt};
use futures_signals::map_ref;

type AnySignal<T> = Pin<Box<dyn Signal<Item=T> + Send + Sync>>;
type SignalField<T> = OnceLock<Broadcaster<AnySignal<T>>>;

#[derive(Clone)]
struct Root {
    houses: MutableVec<House>,
    rooms: MutableVec<Room>,
    windows: MutableVec<Window>,
}

impl Root {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            houses: MutableVec::new(),
            rooms: MutableVec::new(),
            windows: MutableVec::new(),
        })
    }
}

#[derive(Clone)]
struct House {
    id: i32,

    total_volume: SignalField<f64>,
    total_window_area: SignalField<f64>,

    root: Weak<Root>,
    rooms: SignalField<Vec<Room>>,
    windows: SignalField<Vec<Window>>,
}

impl House {
    fn new(root: Arc<Root>, id: i32) -> House {
        let me = Self {
            id,
            total_volume: OnceLock::new(),
            total_window_area: OnceLock::new(),
            root: Arc::downgrade(&root),
            rooms: OnceLock::new(),
            windows: OnceLock::new(),
        };
        root.houses.lock_mut().push_cloned(me.clone());
        me
    }

    fn total_volume(&self) -> impl Signal<Item = f64> {
        self.total_volume.get_or_init(|| {
            let signal = self.rooms()
                .to_signal_vec()
                .map_signal(|child| child.volume())
                .sum();
            (Box::pin(signal) as AnySignal<f64>).broadcast()
        }).signal()
    }

    fn total_window_area(&self) -> impl Signal<Item = f64> {
        self.total_window_area.get_or_init(|| {
           let signal = self.windows()
               .to_signal_vec()
               .map_signal(|child| child.area()).sum();
            (Box::pin(signal) as AnySignal<f64>).broadcast()
        }).signal()
    }

    fn rooms(&self) -> impl Signal<Item = Vec<Room>> {
        self.rooms.get_or_init(|| {
            let id = self.id;
            let signal = self.root.upgrade().expect("Root not found!").rooms
                .signal_vec_cloned()
                .filter(move |child| child.parent_id == id)
                .to_signal_cloned();
            (Box::pin(signal) as AnySignal<Vec<Room>>).broadcast()
        }).signal_cloned()
    }

    fn windows(&self) -> impl Signal<Item = Vec<Window>> {
        self.windows.get_or_init(|| {
            let signal = self.rooms()
                .to_signal_vec()
                .map_signal(|room| room.windows())
                .to_signal_cloned()
                .map(|windows| windows.into_iter().flat_map(|windows| windows.into_iter()).collect());
            (Box::pin(signal) as AnySignal<Vec<Window>>).broadcast()
        }).signal_cloned()
    }
}

#[derive(Clone)]
struct Room {
    id: i32,

    parent_id: i32,
    house: SignalField<House>,

    windows: SignalField<Vec<Window>>,

    // inputs
    length: Mutable<f64>,
    width: Mutable<f64>,
    height: Mutable<f64>,

    // calculations
    volume: SignalField<f64>,
    volume_percentage: SignalField<f64>,

    total_window_surface: SignalField<f64>,
    surface: SignalField<f64>,

    root: Weak<Root>,
}

impl Room {
    fn new(root: Arc<Root>, id: i32, parent_id: i32, length: f64, width: f64, height: f64) -> Room {
        let me = Self {
            id,
            parent_id,

            length: Mutable::new(length),
            width: Mutable::new(width),
            height: Mutable::new(height),
            volume: OnceLock::new(),
            volume_percentage: OnceLock::new(),
            total_window_surface: OnceLock::new(),
            surface: OnceLock::new(),

            root: Arc::downgrade(&root),
            house: OnceLock::new(),
            windows: OnceLock::new(),
        };
        root.rooms.lock_mut().push_cloned(me.clone());
        me
    }

    fn length(&self) -> impl Signal<Item = f64> {
        self.length.signal()
    }
    fn width(&self) -> impl Signal<Item = f64> {
        self.width.signal()
    }
    fn height(&self) -> impl Signal<Item = f64> {
        self.height.signal()
    }

    fn volume(&self) -> impl Signal<Item = f64> {
        self.volume.get_or_init(|| {
            let signal = map_ref! {
                let length = self.length(),
                let width = self.width(),
                let height = self.height() => {
                    *length * *width * *height
                }
            };
            (Box::pin(signal) as AnySignal<f64>).broadcast()
        }).signal()
    }

    fn volume_percentage(&self) -> impl Signal<Item = f64> {
        self.volume_percentage.get_or_init(|| {
            let signal = map_ref! {
                let total_volume = self.house().switch(|p| p.total_volume()),
                let volume = self.volume() => {
                    *volume / *total_volume
                }
            };
            (Box::pin(signal) as AnySignal<f64>).broadcast()
        }).signal()
    }

    fn total_window_surface(&self) -> impl Signal<Item = f64> {
        self.total_window_surface.get_or_init(|| {
           let signal = self.windows()
               .to_signal_vec()
               .map_signal(|window| window.area())
               .sum();
            (Box::pin(signal) as AnySignal<f64>).broadcast()
        }).signal()
    }

    fn surface(&self) -> impl Signal<Item = f64> {
        self.surface.get_or_init(|| {
            let signal = map_ref! {
                let height = self.height(),
                let width = self.width(),
                let length = self.length(),
                let total_window_surface = self.total_window_surface() => {
                    *height * *length * 2.
                    + *height * *width * 2.
                    + *width * *length * 2.
                    - *total_window_surface
                }
            };
            (Box::pin(signal) as AnySignal<f64>).broadcast()
        }).signal()
    }

    fn house(&self) -> impl Signal<Item =House> {
        self.house.get_or_init(|| {
            let parent_id = self.parent_id;
            let signal = self.root.upgrade().expect("Root not found!").houses
                .signal_vec_cloned()
                .filter(move |parent| parent.id == parent_id)
                .to_signal_map(|parents| parents.get(0).expect("Parent not found!").clone());
            (Box::pin(signal) as AnySignal<House>).broadcast()
        }).signal_cloned()
    }

    fn windows(&self) -> impl Signal<Item = Vec<Window>> {
        self.windows.get_or_init(|| {
           let id = self.id;
            let signal = self.root.upgrade().expect("Root not found!").windows
                .signal_vec_cloned()
                .filter(move |window| window.room_id == id)
                .to_signal_cloned();
            (Box::pin(signal) as AnySignal<Vec<Window>>).broadcast()
        }).signal_cloned()
    }
}

#[derive(Clone)]
struct Window {
    id: i32,
    room_id: i32,

    width: Mutable<f64>,
    height: Mutable<f64>,

    root: Weak<Root>,
    area: SignalField<f64>,
}

impl Window {
    fn new(root: Arc<Root>, id: i32, room_id: i32, width: f64, height: f64) -> Window {
        let me = Self {
            id,
            room_id,

            width: Mutable::new(width),
            height: Mutable::new(height),

            root: Arc::downgrade(&root),
            area: OnceLock::new(),
        };
        root.windows.lock_mut().push_cloned(me.clone());
        me
    }

    fn width(&self) -> impl Signal<Item = f64> { self.width.signal() }
    fn height(&self) -> impl Signal<Item = f64> { self.height.signal() }

    fn area(&self) -> impl Signal<Item = f64> {
        self.area.get_or_init(|| {
            let signal = map_ref! {
                let width = self.width(),
                let height = self.height() => {
                    *width * *height
                }
            };
            (Box::pin(signal) as AnySignal<f64>).broadcast()
        }).signal()
    }
}

mod tests {
    use std::time::Duration;
    use async_std::task;
    use async_std::task::spawn_local;
    use super::*;

    #[test]
    fn test() {
        let root = Root::new();
        let house = House::new(root.clone(), 1);
        let room1 = Room::new(root.clone(), 1, 1, 1., 1., 2.);
        let room2 = Room::new(root.clone(), 2, 1, 1., 1., 1.);
        let window1 = Window::new(root.clone(), 1, 1, 0.2, 0.2);
        let fut = room1.surface().for_each(|surface| {
            println!("Room 1 surface: {}", surface);
            async {}
        });
        task::spawn(fut);
        std::thread::sleep(Duration::from_secs(1));
        window1.height.set(0.4);
        std::thread::sleep(Duration::from_secs(1));
    }
}
