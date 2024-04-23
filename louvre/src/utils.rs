/// Indicating winding direction of a vertex list
/// 
#[derive(Debug, PartialEq)]
pub enum Winding {
  CCW, CW, Zero,
}


/// Signed area of a polygon;
/// ccw: <0
/// cw: >0
/// or 0;
/// 
pub fn signed_area(data: &Vec<f64>, dim: usize) -> Winding {
  let mut sum = 0 as f64;
  let mut j = data.len()-dim;
  for i in (0..(data.len())).step_by(dim) {
    sum += (data[i]-data[j])*(data[i+1]+data[j+1]);
    j = i;
  }
  if sum>0. {Winding::CW} else if sum<0. {Winding::CCW} else {Winding::Zero}
}

/// Signed area of a triangle
pub fn area(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64,) -> Winding {
  let result = (by-ay)*(cx-bx) - (bx-ax)*(cy-by);
  if result>0. {Winding::CW} else if result<0. {Winding::CCW} else {Winding::Zero}
}