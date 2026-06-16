use gtk4::Application;
use gtk4::prelude::*;

use nterm::strings as s;
use nterm::ui::app::AppCtx;

fn main() {
    let app = Application::builder().application_id(s::APP_ID).build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let ctx = AppCtx::new(app);
    ctx.build();
}
