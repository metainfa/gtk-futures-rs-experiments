extern crate futures;
extern crate gtk;
extern crate tokio_core;
extern crate tokio_timer;

use std::time::Duration;

use futures::{Future, Stream};
use gtk::{Inhibit, WidgetExt, Window, WindowType};
use tokio_core::reactor::Core;
use tokio_timer::Timer;

fn main() {
    gtk::init().unwrap();
    let window = Window::new(WindowType::Toplevel);

    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let mut core = Core::new().unwrap();
    let timer = Timer::default();
    let interval = timer.interval(Duration::from_millis(1000));
    let future = interval.for_each(|_| {
        println!("Interval");
        Ok(())
    });
    /*let sleep = timer.sleep(Duration::from_millis(5000));
    let future = sleep.and_then(|_| {
        println!("Time out");
        let timer = Timer::default();
        timer.sleep(Duration::from_millis(0))
    });*/
    let handle = core.handle();
    handle.spawn(future.map_err(|_| ()));
    loop {
        core.turn(Some(Duration::from_millis(10)));

        if gtk::events_pending() {
            gtk::main_iteration();
        }

    }
    //core.run(sleep).unwrap();
}
