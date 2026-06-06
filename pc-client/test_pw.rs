use pipewire as pw;
fn main() {
    pw::init();
    let mainloop = pw::main_loop::MainLoop::new(None).unwrap();
    let (tx, rx) = pw::channel::channel::<()>();
    let _receiver = rx.attach(&mainloop.loop_(), move |_| {
        mainloop.quit();
    });
}
