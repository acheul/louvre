//! web_sys helper functions

use crate::*;
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d};
use wasm_bindgen::prelude::*;

pub struct DrawLouvre;

impl DrawLouvre {

  /// initiate data
  pub fn init_data(
    data: Option<Vec<f64>>,
    max_w: Option<f64>, max_h: Option<f64>, ratio: Option<f64>
  ) -> Vec<f64> {
    let data = data.unwrap_or(
      vec![684.0, 27.0, 725.0, 335.0, 673.0, 448.0, 507.0, 617.0, 389.0, 688.0, 264.0, 702.0, 96.0, 689.0, 13.0, 651.0, 69.0, 739.0, 327.0, 810.0, 603.0, 805.0, 698.0, 745.0, 778.0, 676.0, 816.0, 587.0, 826.0, 450.0, 829.0, 365.0, 795.0, 243.0]
    );
    // let data = data.unwrap_or(vec![[0., 0.], [0., 3.], [3., 0.], [3., 4.], [-1., 0.]].concat());

    let x1 = data.iter().step_by(2).max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let x0 = data.iter().step_by(2).min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let y1: Option<&f64> = data.iter().skip(1).step_by(2).max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let y0 = data.iter().skip(1).step_by(2).min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    
    let Some(x1) = x1 else { return data };
    let Some(x0) = x0 else { return data };
    let Some(y1) = y1 else { return data };
    let Some(y0) = y0 else { return data };

    let w_ = x1-x0;
    let h_ = y1-y0;
    if w_.floor()==0. || h_.floor()==0. { return data };
    let (x0, y0) = (*x0, *y0);

    let w = max_w.unwrap_or(gloo_utils::document_element().client_width() as f64);
    let h = max_h.unwrap_or(gloo_utils::document_element().client_height() as f64);
    let ratio = ratio.unwrap_or(0.9);
    let wr = w/w_ * ratio;
    let hr = h/h_ * ratio;
    let r = wr.min(hr);

    let x_margin = (w-w_*r)*0.5;
    let y_margin = (h-h_*r)*0.5;

    data.into_iter().enumerate().map(|(i, x)| {
      if i%2==0 { (x-x0)*r+x_margin } else { (x-y0)*r+y_margin }
    }).collect()
  }

  pub fn set_canvas_size<E: AsRef<HtmlCanvasElement>>(canvas: E, width: u32, height: u32) {
    canvas.as_ref().set_width(width);
    canvas.as_ref().set_height(height);
  }

  pub fn set_canvas_window_size<E: AsRef<HtmlCanvasElement>>(canvas: E) {
    let w = gloo_utils::window().inner_width().unwrap_throw().as_f64().unwrap();
    let h = gloo_utils::window().inner_height().unwrap_throw().as_f64().unwrap();
    Self::set_canvas_size(canvas, w as u32, h as u32);
  }

  pub fn clear_canvas<E: AsRef<HtmlCanvasElement>>(canvas: E) {
    let context = Self::get_context(canvas.as_ref());
    let rect = canvas.as_ref().get_bounding_client_rect();
    context.clear_rect(0.,0., rect.width(), rect.height());
  }

  pub fn get_context<E: AsRef<HtmlCanvasElement>>(canvas: E) -> CanvasRenderingContext2d {
    canvas.as_ref().get_context("2d")
    .unwrap()
    .unwrap()
    .dyn_into::<web_sys::CanvasRenderingContext2d>()
    .unwrap()
  }

  pub fn random_rgb(i: Option<usize>, alpha: Option<f64>) -> String {

    let alpha = alpha.unwrap_or(0.5);
    let i = i.unwrap_or((js_sys::Math::random()*255.0) as usize);
    let i = i + ((js_sys::Math::random()*20.) as usize);
    let d1 = (js_sys::Math::random()*20.) as usize + 50;
    let d2 = (js_sys::Math::random()*20.) as usize + 100;
    let r = i%255;
    let g = (i+d1)%255;
    let b = (i+d2)%255;
    return format!("rgba({},{},{},{:.1})", r, g, b, alpha);
  }

  pub fn draw_outline<E: AsRef<HtmlCanvasElement>>(
    canvas: E,
    data: &Vec<f64>,
    stroke_style: &str
  ) {
    let context = Self::get_context(canvas);

    context.set_stroke_style(&JsValue::from_str(stroke_style));

    if data.len()>2 {
      context.begin_path();
      context.move_to(data[0], data[1]);
      for i in (2..(data.len())).step_by(2) {
        let x = data[i];
        let y = data[i+1];
        context.line_to(x, y);
        context.stroke();

        context.begin_path();
        context.move_to(x, y);
      }
      let (x, y) = (data[0], data[1]);
      context.line_to(x, y);
      context.stroke();
    }
  }

  pub fn draw_triangles<E: AsRef<HtmlCanvasElement>>(
    canvas: E,
    new_data: &Vec<f64>,
    indices: &Vec<usize>,
    fill_style: Option<&str>,
    stroke_style: Option<&str>,
    clean_former: bool
  ) {
    let rand_fill = move |i: usize| {
      Self::random_rgb(Some(i+100), Some(0.8))
    };

    let context = Self::get_context(canvas.as_ref());

    if clean_former {
      let rect = canvas.as_ref().get_bounding_client_rect();
      context.clear_rect(0.,0., rect.width(), rect.height());
    }

    fill_style.map(|x| context.set_fill_style(&JsValue::from_str(x)));
    stroke_style.map(|x| context.set_stroke_style(&JsValue::from_str(x)));

    for (e, i) in (0..(indices.len())).step_by(3).enumerate() {

      if fill_style.is_none() {
        context.set_fill_style(&JsValue::from_str(&rand_fill(e)));
      }
      context.begin_path();

      let (a, b, c) = (indices[i], indices[i+1], indices[i+2]);
      context.move_to(new_data[a*2], new_data[a*2+1]);
      context.line_to(new_data[b*2], new_data[b*2+1]);
      context.line_to(new_data[c*2], new_data[c*2+1]);
      context.fill();
      if stroke_style.is_some() { context.stroke() };
    }
  }

  pub fn triangulate_and_draw<E: AsRef<HtmlCanvasElement>>(
    canvas: E,
    mut data: Vec<f64>,
    fill_style: Option<&str>,
    stroke_style: Option<&str>,
    clean_former: bool
  ) {
    if data.len()<6 {
      return;
    }
    let (new_data, indices) = triangulate(&mut data, 2);
    Self::draw_triangles(canvas, &new_data, &indices, fill_style, stroke_style, clean_former);
  }
}