use sycamore::prelude::*;
use sycamore::futures::spawn_local_scoped;
use louvre::triangulate;
use gloo_console::Timer;
// use gloo_console::log;


fn main() {
  sycamore::render(App);
}


#[component]
pub fn App<G: Html>() -> View<G> {

  on_mount(move || {
    spawn_local_scoped(async move {
      performance_check_triangulate().await;
    });
  });

  view! {
    main() {
      "Hello Sycamore!"
    }
  }
}


async fn load_json(url: &str) -> Vec<f64> {
  let a: Vec<Vec<Vec<f64>>> = gloo_net::http::Request::get(url).send().await.unwrap()
    .json().await.unwrap();
  let a: Vec<f64> = a[0].concat();
  a
}


async fn performance_check_triangulate() {
  let poly_files = vec!["hilbert", "water2", "inter1", "inter2", "inter3", "inter4"];
  for f in poly_files {
    let mut a = load_json(&format!("/assets/{}.json", f)).await;

    let t = 100;
    let _timer = Timer::scope(&format!("{}", f), || {
      for _i in 0..t {
        let (_new_data, _indices) = triangulate(&mut a, 2);
      }
    });
  }
}