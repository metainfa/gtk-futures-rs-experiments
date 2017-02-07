extern crate crossbeam;
extern crate futures;
extern crate gtk;
extern crate tokio_core;
extern crate tokio_timer;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crossbeam::sync::MsQueue;
use futures::{Async, Future, Poll, Stream};
use futures::task::{self, Task};
use gtk::{Button, ButtonExt, ContainerExt, Inhibit, Label, WidgetExt, Window, WindowType};
use gtk::Orientation::Vertical;
use tokio_core::reactor::Core;
use tokio_timer::Timer;

use self::Msg::*;

#[derive(Clone, Debug)]
enum Msg {
    Decrement,
    Increment,
    Quit,
}

#[derive(Clone)]
struct EventStream<T> {
    events: Rc<MsQueue<T>>,
    task: Rc<RefCell<Option<Task>>>,
}

impl<T> EventStream<T> {
    fn new() -> Self {
        EventStream {
            events: Rc::new(MsQueue::new()),
            task: Rc::new(RefCell::new(None)),
        }
    }

    fn emit(&self, event: T) {
        if let Some(ref task) = *self.task.borrow() {
            task.unpark();
        }
        self.events.push(event);
    }

    /*fn get_event(&self) -> Option<T> {
        self.events.try_pop()
    }*/
}

impl<T> Stream for EventStream<T> {
    type Item = T;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.events.try_pop() {
            Some(event) => {
                *self.task.borrow_mut() = None;
                Ok(Async::Ready(Some(event)))
            },
            None => {
                *self.task.borrow_mut() = Some(task::park());
                Ok(Async::NotReady)
            },
        }
    }
}

fn main() {
    gtk::init().unwrap();

    let stream = EventStream::new();

    let window = Window::new(WindowType::Toplevel);

    let vbox = gtk::Box::new(Vertical, 0);
    window.add(&vbox);

    let plus_button = Button::new_with_label("+");
    vbox.add(&plus_button);
    {
        let stream = stream.clone();
        plus_button.connect_clicked(move |_| {
            stream.emit(Increment);
        });
    }

    let label = Label::new(Some("0"));
    vbox.add(&label);

    let minus_button = Button::new_with_label("-");
    vbox.add(&minus_button);
    {
        let stream = stream.clone();
        minus_button.connect_clicked(move |_| {
            stream.emit(Decrement);
        });
    }

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
