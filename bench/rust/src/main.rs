use louvre::*;
use std::time::Instant;

fn main() {
  performance_check_triangulate();
}

fn performance_check_triangulate() {
  let poly_files = vec!["hilbert", "water2", "inter1", "inter2", "inter3", "inter4"];
  for f in poly_files {
    let data = std::fs::read_to_string(format!("../../assets/{}.json", f)).unwrap();
    let a: Vec<Vec<Vec<f64>>> = serde_json::from_str(&data).unwrap();
    let mut a: Vec<f64> = a[0].concat();

    let t = 100;
    let now = Instant::now();
    for _i in 0..t {
      let (_new_data, _indices) = triangulate(&mut a, 2);
    }
    let time = now.elapsed().as_millis();
    let r = format!("{}tries: {}: {}(ms)", t, f, time);
    println!("{}", &r);
  }
}