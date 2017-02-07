extern crate crossbeam;
extern crate futures;
extern crate gtk;
extern crate tokio_core;
extern crate tokio_timer;

use std::cell::{Cell, RefCell};
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
struct QuitFuture {
    quitted: Rc<Cell<bool>>,
}

impl QuitFuture {
    fn new() -> Self {
        QuitFuture {
            quitted: Rc::new(Cell::new(false)),
        }
    }

    fn quit(&self) {
        self.quitted.set(true);
    }
}

impl Future for QuitFuture {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        if self.quitted.get() {
            Ok(Async::Ready(()))
        }
        else {
            Ok(Async::NotReady)
        }
    }
}

#[derive(Clone)]
struct EventStream<T, W: Clone> {
    events: Rc<MsQueue<T>>,
    task: Rc<RefCell<Option<Task>>>,
    widgets: W,
}

impl<T, W: Clone> EventStream<T, W> {
    fn new(widgets: W) -> Self {
        EventStream {
            events: Rc::new(MsQueue::new()),
            task: Rc::new(RefCell::new(None)),
            widgets: widgets,
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

impl<T, W: Clone> Stream for EventStream<T, W> {
    type Item = (T, W);
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.events.try_pop() {
            Some(event) => {
                *self.task.borrow_mut() = None;
                Ok(Async::Ready(Some((event, self.widgets.clone()))))
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

    let vbox = gtk::Box::new(Vertical, 0);

    let plus_button = Button::new_with_label("+");
    vbox.add(&plus_button);

    let label = Label::new(Some("0"));
    vbox.add(&label);

    let stream = EventStream::new(label);
    let mut quit_future = QuitFuture::new();

    let window = Window::new(WindowType::Toplevel);

    window.add(&vbox);

    {
        let stream = stream.clone();
        plus_button.connect_clicked(move |_| {
            stream.emit(Increment);
        });
    }

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

    let event_future = {
        let quit_future = quit_future.clone();
        stream.for_each(move |(event, label)| {
            fn adjust(label: Label, delta: i32) {
                if let Some(text) = label.get_text() {
                    let num: i32 = text.parse().unwrap();
                    let result = num + delta;
                    label.set_text(&result.to_string());
                }
            }

            match event {
                Decrement => adjust(label, -1),
                Increment => adjust(label, 1),
                Quit => quit_future.quit(),
            }
            Ok(())
        })
    };

    let future = interval.join(event_future).map(|_| ());

    let handle = core.handle();
    handle.spawn(future);
    while quit_future.poll() == Ok(Async::NotReady) {
        core.turn(Some(Duration::from_millis(10)));

        if gtk::events_pending() {
            gtk::main_iteration();
        }
    }
}
