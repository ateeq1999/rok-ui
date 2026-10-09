// A bloc that stores another bloc's handle: rejected, because handles are not `Send`. React
// to the other bloc with a `BlocListener` in the view instead.
use rok_ui_bloc::{Bloc, BlocHandle, Emitter};

struct Inner;

impl Bloc for Inner {
    type Event = ();
    type State = u8;
    fn initial_state(&self) -> u8 {
        0
    }
    async fn on(&self, _event: (), _emit: &Emitter<u8>) {}
}

struct Outer {
    inner: BlocHandle<Inner>,
}

impl Bloc for Outer {
    type Event = ();
    type State = u8;
    fn initial_state(&self) -> u8 {
        self.inner.state()
    }
    async fn on(&self, _event: (), _emit: &Emitter<u8>) {}
}

fn main() {}
