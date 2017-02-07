extern crate crossbeam;
extern crate futures;
extern crate gtk;
extern crate tokio_core;
extern crate tokio_timer;

use std::rc::Rc;
use std::time::Duration;

use crossbeam::sync::MsQueue;
use futures::{Async, Future, Poll, Stream};
use gtk::{Button, ButtonExt, ContainerExt, Inhibit, WidgetExt, Window, WindowType};
use tokio_core::reactor::Core;
use tokio_timer::Timer;

use self::Msg::*;

#[derive(Clone, Debug)]
enum Msg {
    Clicked,
    Quit,
}

#[derive(Clone)]
struct EventStream<T> {
    events: Rc<MsQueue<T>>,
}

impl<T> EventStream<T> {
    fn new() -> Self {
        EventStream {
            events: Rc::new(MsQueue::new()),
        }
    }

    fn emit(&self, event: T) {
        self.events.push(event);
    }

    fn get_event(&self) -> Option<T> {
        self.events.try_pop()
    }
}

impl<T> Stream for EventStream<T> {
    type Item = T;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.events.try_pop() {
            Some(event) => Ok(Async::Ready(Some(event))),
            None => Ok(Async::NotReady),
        }
    }
}

fn main() {
    gtk::init().unwrap();

    let stream = EventStream::new();

    let window = Window::new(WindowType::Toplevel);

    let button = Button::new_with_label("Click me!");
    {
        let stream = stream.clone();
        button.connect_clicked(move |_| {
            stream.emit(Clicked);
        });
    }
    window.add(&button);

    window.show_all();

    {
        let stream = stream.clone();
        window.connect_delete_event(move |_, _| {
            stream.emit(Quit);
            Inhibit(false)
        });
    }

    let mut core = Core::new().unwrap();
    let timer = Timer::default();
    let interval = timer.interval(Duration::from_millis(1000));
    let interval = interval.map_err(|_| ()).for_each(|_| {
        println!("Interval");
        Ok(())
    });

    let event_future = stream.for_each(|event| {
        println!("{:?}", event);
        Ok(())
    });

    let future = interval.join(event_future).map(|_| ());

    let handle = core.handle();
    handle.spawn(future);
    loop {
        core.turn(Some(Duration::from_millis(0)));

        if gtk::events_pending() {
            gtk::main_iteration();
        }
    }
}
