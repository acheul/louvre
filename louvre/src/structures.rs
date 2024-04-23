//! Data structures for triangulation
//! 
//! Lots of unsafe codes and raw pointers are used to implement linked list.
//! They went through rust's miri test and they're designed to be safe.

use crate::*;


/// Vertex is used to makes initial linked nodes from a coordinates input.
/// * Each Vertex is a "node" which contains node number(i), coordinates(x, y), bbox information(top, bottom, left, right) and some others.
/// * Technically, each Vertex is a "segment" at the same time.
///   Coordinates(x, y) of each vertex are coordinates of starting point of the segment.
/// 
/// * Fields `topdown`, `top`, `bottom`, `left` and `right` are used to boost up "intersection check". 
///   Each Vertices are linked to their own previous and next vertices.
///   The linking is implemented via the raw pointer of Rust. Hopefully, it's designed to be memory safe.
pub struct Vertex<'a> {
  pub i: usize,
  pub x: f64,
  pub y: f64,
  pub topdown: bool,
  pub top: f64,
  pub bottom: f64,
  pub left: f64,
  pub right: f64,
  pub sign: bool,
  pub valid: bool,
  pub sects: Option<Vec<*mut Sect<'a>>>,
  pub prev: *mut Vertex<'a>,
  pub next: *mut Vertex<'a>,
  pub next_sect: *mut Sect<'a>,
}


/// Sect handles intersection points which are generated from intersecting segments.
pub struct Sect<'a> {
  pub i: usize,
  pub x: f64,
  pub y: f64,
  pub dual: *mut Sect<'a>,
  pub next: *mut Sect<'a>,
  pub other: *mut Vertex<'a>,
  pub sign: bool,
  pub valid: bool,
}


/// RedunSect is devised to handle a tricky case: when multiple intersection points have same coordinates.
/// It might sound weird, but happens.
#[derive(Debug)]
pub struct RedunSect {
  pub i: usize,
  /// direction
  pub dir: bool,
  pub angle: f64,
  /// 기준segment에서 그대로 뻗어나가는지 여부.
  /// If a proposed direction grows straight from the key segment or not.
  pub is_straight: bool,
}

pub struct SimpleCycle<'a> {
  pub point: *mut Point<'a>,
  pub len: usize,
}

pub struct Point<'a> {
  pub i: usize,
  pub x: f64,
  pub y: f64,
  pub reflex: bool,
  pub prev: *mut Point<'a>,
  pub next: *mut Point<'a>,
}


impl Vertex<'_> {
  /// Returns new Vertex.
  /// Updates its bbox fields(topdown, top, bottom, left, right) from the beginning.
  pub fn new(i: usize, x0: f64, y0: f64, x1: f64, y1: f64, last: *mut Vertex) -> *mut Vertex {
    let mut topdown = true;
    let mut top = y0;
    let mut bottom = y1;
    let mut left = x0;
    let mut right = x1;

    if y1>y0 {
      topdown = false;
      top = y1;
      bottom = y0;
    } else if y1==y0 {
      if x0>x1 {
        topdown = false;
      }
    }
    if x0>x1 {
      right = x0;
      left = x1;
    }

    let v = Box::into_raw(Box::new(Vertex{i: i, x: x0, y: y0,
      topdown: topdown,
      top: top,
      bottom: bottom,
      left: left,
      right: right,
      sign: true,
      valid: true,
      sects: None,
      prev: ptr::null_mut(),
      next: ptr::null_mut(),
      next_sect: ptr::null_mut(),
    }));
    unsafe {
      if last.is_null() {
        (*v).prev = v;
        (*v).next = v;
      } else {
        (*v).next = (*last).next;
        (*v).prev = last;
        (*(*last).next).prev = v;
        (*last).next = v;
      }
    }
    return v;
  }

  pub fn equals(&self, other: &Self) -> bool {
    if (self.x==other.x) & (self.y==other.y) {
      true
    } else {
      false
    }
  }

  pub fn is_adjacent(&self, other: &Self, len: usize) -> bool {
    let i = self.i;
    let j = other.i;
    if (i==0) | (j==0) {
      if i + j == len-1 {
        return true;
      }
    }
    let abs = if i>j {i-j} else {j-i};
    if abs==1 {
      return true;
    }
    false
  }
}

impl PartialEq<Vertex<'_>> for Vertex<'_> {
  fn eq(&self, other: &Vertex) -> bool {
    self.x==other.x && self.y==other.y && self.i==other.i
  }
}

impl PartialOrd for Vertex<'_> {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {

    let cmpy = (&self.y).partial_cmp(&other.y); // larger-y priority
    match cmpy {
      Some(Ordering::Equal) => {
        let cmpx = (&other.x).partial_cmp(&self.x); // smaller-x priority
        match cmpx {
          Some(Ordering::Equal) => {
            // When tow vertices share same x,y coords, compare their ... (잘 생각해봅시다)
            // DISCLAIMER: At this moment, this particular ordering logic is not required.
            // I keep this comment in case of future development.
            (&other.i).partial_cmp(&self.i) // smaller-i priority; The last and first should be different.
          },
          _ => cmpx,
        }
      },
      _ => cmpy,
    }
  }
}

impl PartialEq<Sect<'_>> for Sect<'_> {
  fn eq(&self, other: &Sect) -> bool {
    self.x==other.x && self.y==other.y
  }
}

impl PartialOrd for Sect<'_> {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {

    let cmpy = (&self.y).partial_cmp(&other.y); // larger-y priority
    match cmpy {
      Some(Ordering::Equal) => {
        (&other.x).partial_cmp(&self.x) // smaller-x priority
      },
      _ => cmpy,
    }
  }
}

impl RedunSect {
  /// Make two new RedunSects.
  /// vx,vy: key segments vertex; px,py: intersection point; ox,oy: intersected segment's next coords.
  pub fn new(i: usize, vx:f64, vy:f64, px:f64, py:f64, ox:f64, oy:f64) -> (RedunSect, RedunSect) {
    let local_wind = area(vx, vy, px, py, ox, oy);
    let a2 = (ox-px).powi(2) + (oy-py).powi(2);
    let b2 = (vx-px).powi(2) + (vy-py).powi(2);
    let c2 = (ox-vx).powi(2) + (oy-vy).powi(2);
    let mut deno = 2.*(a2.sqrt())*(b2.sqrt());
    if deno==0. {deno+=1e-10};
    let cos_c: f64 = (a2+b2-c2) / deno;
    let mut angle: f64 = cos_c.acos();
    if local_wind==Winding::CW { // CCW(left turn)을 기준으로 삼기.
      angle = std::f64::consts::PI*2.-angle;
    }
    let mut angle2 = angle + std::f64::consts::PI;
    if angle2 > std::f64::consts::PI*2. {
      angle2 -= std::f64::consts::PI*2.;
    }

    let r1 = RedunSect{ i: i, dir: true, angle: angle, is_straight: false };
    let r2 = RedunSect{ i: i, dir: false, angle: angle2, is_straight: false };
    (r1, r2)
  }
}

impl PartialEq<RedunSect> for RedunSect {
  fn eq(&self, other: &RedunSect) -> bool {
    self.angle == other.angle
  }
}

impl PartialOrd for RedunSect {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    let cmp = (&self.angle).partial_cmp(&other.angle);
    match cmp {
      Some(Ordering::Equal) => {
        if self.dir {
          if !other.dir { Some(Ordering::Greater) } else {
            Some(Ordering::Equal)
          }
        } else {
          if other.dir { Some(Ordering::Less) } else {
            Some(Ordering::Equal)
          }
        }
      },
      _ => cmp
    }
  }
}

impl Point<'_> {
  pub fn new<'a>(i:usize, x:f64, y:f64, last: *mut Point<'a>) -> *mut Point<'a> {
    let p = Box::into_raw(Box::new(
      Point{ i:i, x:x, y:y, reflex:true, prev:ptr::null_mut(), next:ptr::null_mut() }
    ));
    unsafe {
      if last.is_null() {
        (*p).prev = p;
        (*p).next = p;
      } else {
        (*p).next = (*last).next;
        (*p).prev = last;
        (*(*last).next).prev = p;
        (*last).next = p;
      }
    }
    return p;
  }
}
