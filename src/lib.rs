use std::{borrow::Cow, rc::Rc, sync::Arc};

use gpui::{prelude::*, *};
use wasm_bindgen::prelude::*;

const PAPER: u32 = 0xf4efe6;
const INK: u32 = 0x2b2622;
const MUTED: u32 = 0x6b635a;
const FAINT: u32 = 0x8a8178;

struct LandingPage;

impl Render for LandingPage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .size_full()
            .bg(rgb(PAPER))
            .text_color(rgb(INK))
            .font_family("Noto Sans SC")
            .child(
                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .gap(px(20.))
                    .px(px(16.))
                    .py(px(32.))
                    .child(
                        img(Arc::new(Image::from_bytes(
                            ImageFormat::Png,
                            include_bytes!("../static/images/logo.png").to_vec(),
                        )))
                        .size(px(96.))
                        .rounded(px(21.))
                        .object_fit(ObjectFit::Contain),
                    )
                    .child(
                        div()
                            .text_size(px(56.))
                            .line_height(relative(1.))
                            .font_weight(FontWeight::MEDIUM)
                            .child("teshi"),
                    )
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(rgb(MUTED))
                            .child("A I   T E S T I N G   A G E N T"),
                    ),
            )
            .child(
                div()
                    .id("github-link")
                    .absolute()
                    .right(px(20.))
                    .bottom(px(16.))
                    .cursor_pointer()
                    .text_size(px(12.))
                    .text_color(rgb(FAINT))
                    .hover(|style| style.text_color(rgb(INK)))
                    .on_click(|_, _, cx| cx.open_url("https://github.com/teshi-org/teshi"))
                    .child("g i t h u b"),
            )
    }
}

#[wasm_bindgen]
pub fn run() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    gpui_platform::web_init();

    let app = gpui_platform::single_threaded_web();

    // GPUI's web application currently owns its state through an Rc. Keep one
    // strong reference alive for the lifetime of the page, matching upstream's
    // story-web bootstrap.
    struct WasmApplication(Rc<AppCell>);
    let wasm_app = unsafe { std::mem::transmute::<Application, WasmApplication>(app) };
    std::mem::forget(wasm_app.0.clone());
    let app = unsafe { std::mem::transmute::<WasmApplication, Application>(wasm_app) };

    app.run(|cx: &mut App| {
        let font =
            Cow::Borrowed(include_bytes!("../assets/NotoSansSC-Regular-subset.ttf").as_slice());
        cx.text_system()
            .add_fonts(vec![font])
            .expect("failed to load bundled font");

        cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| LandingPage))
            .expect("failed to open the teshi window");
        cx.activate(true);
    });

    Ok(())
}
