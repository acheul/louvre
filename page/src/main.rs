use sycamore::prelude::*;
use sycamore::futures::spawn_local_scoped;
use louvre::html::DrawLouvre;
use web_sys::MouseEvent;
use gloo_timers::future::TimeoutFuture;
use webtric::WindowResizing;
// use gloo_console::log;

fn main() {
  sycamore::render(App);
}


#[component]
pub fn App<G: Html>() -> View<G> {

  let window_resizing = WindowResizing::init();

  let outline = create_signal(true);
  let stroke = create_signal(true);
  let random = create_signal(false);
  let assets = create_signal(false);

  let rf_canvas = create_node_ref();
  let get_canvas = move || {
    rf_canvas.try_get::<DomNode>().map(|x| x.unchecked_into::<web_sys::HtmlCanvasElement>())
  };

  // canvas
  let src = DrawLouvre::init_data(None, None, None, None);
  let data = create_signal(src.clone());
  let src = create_signal(src);

  let stroke_style = "rgba(255, 255, 255, 0.8)";
  let outline_style = "rgba(50, 50, 50, 0.8)";

  let stroke_style_ = move || {
    if stroke.get() { Some(stroke_style) } else { None }
  };

  let draw_ = move || {
    if let Some(canvas) = get_canvas() {
      DrawLouvre::triangulate_and_draw(&canvas, data.get_clone(), None, stroke_style_(), true);
      if outline.get() {
        DrawLouvre::draw_outline(&canvas, &data.get_clone(), outline_style);
      }
    }
  };

  let initiate_ = move || {
    data.update(|x| x.clear());
    if let Some(canvas) = get_canvas() {
      DrawLouvre::clear_canvas(canvas);
    }
  };

  // init
  on_mount(move || {
    if let Some(canvas) = get_canvas() {
      DrawLouvre::set_canvas_window_size(canvas);
    }
    draw_();

    create_effect(on((outline, stroke), move || {
      draw_();
    }));

    create_effect(on(window_resizing, move || {
      if window_resizing.get() {
        let data_ = DrawLouvre::init_data(Some(data.get_clone()), None, None, None);
        if let Some(canvas) = get_canvas() {
          DrawLouvre::clear_canvas(&canvas);
          DrawLouvre::set_canvas_window_size(&canvas);
        }
        data.set(data_);
        draw_();
      }
    }));
  });

  // on click
  let mut init = false;
  let on_click = move |e: web_sys::MouseEvent| {
    if !init {
      init = true;
      initiate_();
    }

    let (x, y) = (e.offset_x() as f64, e.offset_y() as f64);
    data.update(|data| { data.push(x); data.push(y); });

    draw_();
  };

  // on ctxmenu // initiation
  let on_ctxmenu = move |e: MouseEvent| {
    e.stop_propagation();
    e.prevent_default();
    initiate_();
  };


  // random
  fn random_drawing(
    data: Signal<Vec<f64>>,
    random: Signal<bool>,
    draw_: impl Fn() + 'static,
    src: Signal<Vec<f64>>,
    step: Option<usize>
  ) {
    
    let step = step.unwrap_or(0);
    let (x, y) = src.with(|src| {
      let len = src.len();
      let step = step%len;
      (src[step], src[step+1])
    });

    let x: f64 = x + js_sys::Math::random()*50.-25.;
    let y = y + js_sys::Math::random()*50.-25.;

    spawn_local_scoped(async move {
      data.update(|data| { data.push(x); data.push(y); });
      draw_();

      TimeoutFuture::new(200+data.with(|x| x.len() as u32)).await;

      if random.get() && data.with(|x| x.len()<100) {
        random_drawing(data, random, draw_, src, Some(step+2));
      } else {
        random.set(false);
      }
    });
  }

  on_mount(move || {
    create_effect(on(random, move || {
      if random.get() {
        initiate_();
        random_drawing(data, random, draw_, src, None);
      }
    }));
  });

  
  // assets
  async fn load_asset(url: &str) -> Vec<f64> {
    let a: Vec<Vec<Vec<f64>>> = gloo_net::http::Request::get(url).send().await.unwrap()
      .json().await.unwrap();
    let a: Vec<f64> = a[0].concat();
    a
  }
  let asset_files = create_signal((0, vec!["inter1", "inter2", "inter3", "inter4", "water2", "hilbert"]));

  on_mount(move || {
    create_effect(on(assets, move || {
      spawn_local_scoped(async move {
        if assets.get() {
          let f = asset_files.update(|(i, ff)| {
            let f = ff[*i];
            *i = (*i+1)%ff.len();
            f
          });
  
          let data_ = load_asset(&format!("/louvre/assets/{}.json", f)).await;
          let data_ = DrawLouvre::init_data(Some(data_), None, None, Some(0.4));
          data.set(data_);
          draw_();
        }
      });
    }));
  });

  
  view! {
    main(
      class="full", style="user-select: none;",
      on:click=on_click, on:contextmenu=on_ctxmenu
    ) {

      div(style="position: absolute; width: 100%; height: 100%; z-index: -1;") {
        canvas(id="canvas", ref=rf_canvas)
      }

      div(style="margin: 16px;") {
        h1() {"🌙 LOUVRE"}
        div(style="display: flex; align-items: center;") {
          a(href="https://crates.io/crates/louvre", title="crate") {
            img(src="https://img.shields.io/crates/v/louvre?style=flat-square")
          }
          a(style="margin-left: 8px;", href="https://docs.rs/louvre", title="docs") {
            img(src="https://img.shields.io/docsrs/louvre?color=skyblue&style=flat-square")
          }
          a(href="https://github.com/acheul/louvre", title="repository") {
            svg(style="width: 1rem; margin-left: 8px;", xmlns="http://www.w3.org/2000/svg", viewBox="0 0 496 512", data-src="Font Awesome Free 6.5.2 by @fontawesome - https://fontawesome.com License - https://fontawesome.com/license/free Copyright 2024 Fonticons, Inc.") {
              path(d="M165.9 397.4c0 2-2.3 3.6-5.2 3.6-3.3 .3-5.6-1.3-5.6-3.6 0-2 2.3-3.6 5.2-3.6 3-.3 5.6 1.3 5.6 3.6zm-31.1-4.5c-.7 2 1.3 4.3 4.3 4.9 2.6 1 5.6 0 6.2-2s-1.3-4.3-4.3-5.2c-2.6-.7-5.5 .3-6.2 2.3zm44.2-1.7c-2.9 .7-4.9 2.6-4.6 4.9 .3 2 2.9 3.3 5.9 2.6 2.9-.7 4.9-2.6 4.6-4.6-.3-1.9-3-3.2-5.9-2.9zM244.8 8C106.1 8 0 113.3 0 252c0 110.9 69.8 205.8 169.5 239.2 12.8 2.3 17.3-5.6 17.3-12.1 0-6.2-.3-40.4-.3-61.4 0 0-70 15-84.7-29.8 0 0-11.4-29.1-27.8-36.6 0 0-22.9-15.7 1.6-15.4 0 0 24.9 2 38.6 25.8 21.9 38.6 58.6 27.5 72.9 20.9 2.3-16 8.8-27.1 16-33.7-55.9-6.2-112.3-14.3-112.3-110.5 0-27.5 7.6-41.3 23.6-58.9-2.6-6.5-11.1-33.3 2.6-67.9 20.9-6.5 69 27 69 27 20-5.6 41.5-8.5 62.8-8.5s42.8 2.9 62.8 8.5c0 0 48.1-33.6 69-27 13.7 34.7 5.2 61.4 2.6 67.9 16 17.7 25.8 31.5 25.8 58.9 0 96.5-58.9 104.2-114.8 110.5 9.2 7.9 17 22.9 17 46.4 0 33.7-.3 75.4-.3 83.6 0 6.5 4.6 14.4 17.3 12.1C428.2 457.8 496 362.9 496 252 496 113.3 383.5 8 244.8 8zM97.2 352.9c-1.3 1-1 3.3 .7 5.2 1.6 1.6 3.9 2.3 5.2 1 1.3-1 1-3.3-.7-5.2-1.6-1.6-3.9-2.3-5.2-1zm-10.8-8.1c-.7 1.3 .3 2.9 2.3 3.9 1.6 1 3.6 .7 4.3-.7 .7-1.3-.3-2.9-2.3-3.9-2-.6-3.6-.3-4.3 .7zm32.4 35.6c-1.6 1.3-1 4.3 1.3 6.2 2.3 2.3 5.2 2.6 6.5 1 1.3-1.3 .7-4.3-1.3-6.2-2.2-2.3-5.2-2.6-6.5-1zm-11.4-14.7c-1.6 1-1.6 3.6 0 5.9 1.6 2.3 4.3 3.3 5.6 2.3 1.6-1.3 1.6-3.9 0-6.2-1.4-2.3-4-3.3-5.6-2z")
            }
          }
        }
        br()
        div() {
          div() { "Click random points to draw triangulated polygon. Right click to initiate." }
          div() { "Louvre can triangulate self-intersecting polygons. Even ones with redundant intersection points." }
          div() { "This is robust." }
        }
        br()
        div(style="display: flex; align-items: center;") {
          ConfigButton(signal=outline, literal="outline")
          ConfigButton(signal=stroke, literal="stroke")
          ConfigButton(signal=random, literal="random")
          ConfigButton2(signal=assets)
        }
      }
    }
  }
}

#[component(inline_props)]
fn ConfigButton<G: Html>(signal: Signal<bool>, literal: &'static str) -> View<G> {

  let rf = create_node_ref();
  on_mount(move || {
    create_effect(on(signal, move || {
      if let Some(rf) = rf.try_get::<DomNode>() {
        if signal.get() {
          rf.remove_class("off");
        } else {
          rf.add_class("off");
        }
      }
    }));
  });

  view! {
    button(ref=rf, class="config", on:click=move |e: web_sys::MouseEvent| {
      signal.set(!signal.get());
      e.stop_propagation();
    }, title=format!("turn {} {}", if signal.get(){"off"}else{"on"}, literal)) {
      (if signal.get(){"⏸️ "} else{"▶️ "}) (literal)
    }
  }
}


#[component(inline_props)]
fn ConfigButton2<G: Html>(signal: Signal<bool>) -> View<G> {

  view! {
    button(class="config", on:click=move |e: web_sys::MouseEvent| {
      signal.set(true);
      e.stop_propagation();
    }, title="Triangulate example polygons") {
      "🎨 examples"
    }
  }
}