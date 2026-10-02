use rok_ui::prelude::*;

#[derive(Clone, Copy)]
enum Tone {
    Calm,
    Loud,
}

styles! {
    CARD = {
        tone(Tone): {
            Calm: { background: muted },
        },
    }
}

fn main() {
    let _ = CARD.tone(Tone::Loud);
}
