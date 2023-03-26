use crate::polygon::utils::{Winding, signed_area, area};
use std::ptr;
use std::cmp::Ordering;
use std::f64;

//-----------------------//
//--- DATA STRUCTURES ---//
//-----------------------//

/// The Vertex struct is used to make initial linked nodes from a coordinates input.
/// Each Vertex is a node which contains node number(i), coordinates(x, y), bbox information(top, bottom, left, right) and some others.
/// Technically, each Vertex is a segment. Coordinates(x, y) of each vertex are coordinates of starting point of the segment.
/// Fields `topdown`, `top`, `bottom`, `left` and `right` are used to boost up intersection check. 
/// Each Vertices are linked to their own previous and next vertices.
/// The linking is implemented via the raw pointer of Rust. Hopefully, it's designed to be memory safe.
pub struct Vertex<'a> {
  i: usize,
  x: f64,
  y: f64,
  topdown: bool,
  top: f64,
  bottom: f64,
  left: f64,
  right: f64,
  sign: bool,
  valid: bool,
  sects: Option<Vec<*mut Sect<'a>>>,
  prev: *mut Vertex<'a>,
  next: *mut Vertex<'a>,
  next_sect: *mut Sect<'a>,
}

/// The Sect struct handles points generated from intersection between original segments.
pub struct Sect<'a> {
  i: usize,
  x: f64,
  y: f64,
  dual: *mut Sect<'a>,
  next: *mut Sect<'a>,
  other: *mut Vertex<'a>,
  sign: bool,
  valid: bool,
}

/// RedunSect struct is devised to handle a tricky case: when multiple intersected points have same coordinates.
/// In an ideal world that kind of thing wouldn't happen and life gonna be much easier. Oddly, however, that often happens.
#[derive(Debug)]
struct RedunSect {
  i: usize,
  dir: bool, // direction;
  angle: f64,
  is_straight: bool, // 기준segment에서 그대로 뻗어나가는지 여부. | If a proposed direction grows straight from the key segment or not.
}

struct SimpleCycle<'a> {
  point: *mut Point<'a>,
  len: usize,
}

struct Point<'a> {
  i: usize,
  x: f64,
  y: f64,
  reflex: bool,
  prev: *mut Point<'a>,
  next: *mut Point<'a>,
}

impl Vertex<'_> {
  /// Returns new Vertex.
  /// Updates its bbox fields(topdown, top, bottom, left, right) from the beginning.
  fn new(i: usize, x0: f64, y0: f64, x1: f64, y1: f64, last: *mut Vertex) -> *mut Vertex {
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

    let mut v = Box::into_raw(Box::new(Vertex{i: i, x: x0, y: y0,
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

  fn is_adjacent(&self, other: &Self, len: usize) -> bool {
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
            // I keep this comment space in case of future development.
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
  fn new(i: usize, vx:f64, vy:f64, px:f64, py:f64, ox:f64, oy:f64) -> (RedunSect, RedunSect) {
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
  fn new<'a>(i:usize, x:f64, y:f64, last: *mut Point<'a>) -> *mut Point<'a> {
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


//---------------------//
//--- IMPLEMETATION ---//
//---------------------//

/// Triangulates a given polygon coordinates.
/// Returns a new coordinates array and a index array of it.
/// 
/// # Arguments
/// * `data` - A vector with float64 which is a coordinates array of a certain polygon.
/// If a polygon has 4 points of [P0(0,0), P1(1,0), P2(1,1), P3(0,1)], then the data input of it would be like [0,0, 1,0, 1,1, 0,1].
pub fn triangulate(data: &mut Vec<f64>, dim: usize) -> (Vec<f64>, Vec<usize>){

  // 1. make linked vertex list with ccw-winding.
  let mut array = linked_vertex_array(data, dim);
  
  // Sort the array in refence to 'top' (in descending order).
  // This is to speed up the intersection test. All you need is just a simple 'top' sorting, not some complex priority sorting.
  unsafe {
    array.sort_by(|b, a| (&(*(*a)).top).partial_cmp(&(*(*b)).top).unwrap());
  }

  // 2. update intesection
  let new_data: Vec<f64>;
  let simple_cycles: Vec<SimpleCycle>;
  if update_intersect(&array) {

    // 3. decompose 
    // sort and link Vertex.sects;
    update_sects(&array[0]);
    // decompose into simple polygon cycles
    (new_data, simple_cycles) = decomp_simples(&array);
    
  } else { 
    (new_data, simple_cycles) = decomp_simple(&array);
  }

  // consume raw pointers
  consume_array(&array);

  // 4. do earcut;
  let indices: Vec<usize> = earcut(&simple_cycles);

  (new_data, indices)
}

/// Consume raw pointers;
fn consume_array(array: &Vec<*mut Vertex>) {
  unsafe {
    array.iter().for_each(|a| {
      if let Some(sects) = &(*(*a)).sects {
        sects.iter().for_each(|s| { drop(Box::from_raw(*s)); });
      }
      drop(Box::from_raw(*a));
    });
  }
}

// ----- step 4. ----- //

fn is_point_inside(ax:f64,ay:f64, bx:f64,by:f64, cx:f64,cy:f64, px:f64,py:f64) -> bool {
  if ((bx-ax)*(py-by) >= (px-bx)*(by-ay)) &
     ((cx-bx)*(py-cy) >= (px-cx)*(cy-by)) & 
     ((ax-cx)*(py-ay) >= (px-ax)*(ay-cy)) {
    true
  } else {
    false
  }
}

fn is_reflex(prev: *mut Point, v: *mut Point, next: *mut Point) -> bool {
  unsafe {
    // Supposed the sign is true (CCW winding).
    match area((*prev).x, (*prev).y, (*v).x, (*v).y, (*next).x, (*next).y) {
      Winding::CW => true,
      _ => false,
    }
  }
}

/* Is it earcut-t-able? */
fn is_ear(prev: *mut Point, v: *mut Point, next: *mut Point) -> bool {
  unsafe {
    // (1) Is it reflex? If so update the state, if still so, skip it.
    if (*v).reflex {
      if is_reflex(prev, v, next) {
        return false;
      } else {
        (*v).reflex = false;
      }
    }

    // (2) Is any point inside the triangle? check;
    // get bbox
    let (ax, bx, cx) = ((*prev).x, (*v).x, (*next).x);
    let (ay, by, cy) = ((*prev).y, (*v).y, (*next).y);
    let x0 = f64::min(f64::min(ax, bx), cx);
    let x1 = f64::max(f64::max(ax, bx), cx);
    let y0 = f64::min(f64::min(ay, by), cy);
    let y1 = f64::max(f64::max(ay, by), cy);

    let mut p: *mut Point = (*next).next;
    while (*p).i != (*prev).i {
      if (x0<=(*p).x) & ((*p).x<=x1) & (y0<=(*p).y) & ((*p).y<=y1) {
        if is_point_inside(ax,ay, bx,by, cx,cy, (*p).x,(*p).y) {
          return false;
        }
      }
      p = (*p).next;
    }
    return true;
  }
}

/* Do ear-clipping and return index list. */
fn earcut(cycles: &Vec<SimpleCycle>) -> Vec<usize> {
  unsafe {
    let mut indices: Vec<usize> = Vec::new();

    cycles.iter().for_each(|cycle| {
      let mut v: *mut Point = cycle.point;
      let mut prev: *mut Point;
      let mut next: *mut Point;
      let mut stopi = (*v).i;

      while (*(*v).prev).i != (*(*v).next).i {
        prev = (*v).prev;
        next = (*v).next;

        if is_ear(prev, v, next) {
          indices.extend(vec![(*prev).i, (*v).i, (*next).i]);
          (*prev).next = next;
          (*next).prev = prev;
          drop(Box::from_raw(v)); // consume

          v = (*next).next;
          stopi = (*v).i;
          continue;
        }

        v = (*v).next;
        if (*v).i==stopi {
          break;
        }
      }

      // consume
      let i = (*(*v).prev).i;
      if i == (*v).i {
        drop(Box::from_raw(v));
      } else {
        let mut v2: *mut Point;
        loop {
          v2 = (*v).next;
          drop(Box::from_raw(v));
          v = v2;
          if (*v).i == i {
            drop(Box::from_raw(v));
            break;
          }
        }
      }
    });
    
    indices
  }
}


// ----- step 3. ----- //

// non-intersecting vertex link -> point link
fn decomp_simple<'a>(array: &'a Vec<*mut Vertex<'a>>) -> (Vec<f64>, Vec<SimpleCycle<'a>>) {
  unsafe {
    let mut new_data: Vec<f64> = Vec::new();
    let simple_cycles: Vec<SimpleCycle>;
    let len = array.len();
    let mut v: *mut Vertex = array[0];

    while (*v).i != 0 {
      v = (*v).next;
    }

    let mut last: *mut Point = ptr::null_mut();
    let vi = (*v).i;
    loop {
      last = Point::new((*v).i, (*v).x, (*v).y, last);
      new_data.push((*v).x);
      new_data.push((*v).y);
      v = (*v).next;
      if (*v).i == vi {
        break;
      }
    }
    simple_cycles = vec![SimpleCycle{ point: last, len: len/2 },];
    (new_data, simple_cycles)
  }
}

/* decompose into simple polygon cycles. (Simple polygon is non-intersecting polygon.) */
fn decomp_simples<'a>(array: &'a Vec<*mut Vertex<'a>>) -> (Vec<f64>, Vec<SimpleCycle<'a>>) {
  unsafe {
    let mut new_data: Vec<f64> = Vec::new();
    let mut simple_cycles: Vec<SimpleCycle> = Vec::new();
    let mut i: usize = 0;
    let mut v: *mut Vertex;
    let mut s: *mut Sect;

    for e in 0..(array.len()) {
      v = array[e];
      if (*v).valid {
        let mut local_data: Vec<f64> = Vec::new();
        let vi = (*v).i;
        loop {
          local_data.push((*v).x); local_data.push((*v).y);
          (*v).valid = false;

          if (*v).next_sect.is_null() {
            v = (*v).next;
          } else {
            s = (*v).next_sect;
            loop {
              local_data.push((*s).x); local_data.push((*s).y);
              (*(*s).dual).valid = false;

              if (*(*s).dual).next.is_null() {
                v = (*(*s).other).next;
                break;
              } else {
                s = (*(*s).dual).next;
              }
            }
          }
          if (*v).i==vi {
            break;
          }   
        }
        // check winding => make ccw linked points
        // We don't need to check the winding validity because we only checked ones starting from the original vertices.
        let len = local_data.len();
        let mut last: *mut Point;
        match signed_area(&local_data, 2) {
          Winding::Zero => {}, // If a simple polygon has zero signed area, don't need to count it.
          Winding::CCW => {
            last = ptr::null_mut();
            for e in (0..len).step_by(2) {
              last = Point::new(i, local_data[e], local_data[e+1], last);
              i += 1;
            }
            new_data.extend(local_data);
            simple_cycles.push(SimpleCycle{ point: last, len: len/2 });
          },
          Winding::CW => {
            let mut new_local_data: Vec<f64> = Vec::new();
            last = ptr::null_mut();
            for e in (0..len).step_by(2).rev() {
              last = Point::new(i, local_data[e], local_data[e+1], last);
              i += 1;
              new_local_data.push(local_data[e]);
              new_local_data.push(local_data[e+1]);
            }
            new_data.extend(new_local_data);
            simple_cycles.push(SimpleCycle{ point: last, len: len/2 });
          },
        }
      }
    }

    // check for remaining Sect -----
    let mut v: *mut Vertex;
    let mut s: *mut Sect;

    for e in 0..(array.len()) {
      v = array[e];
      let vi = (*v).i;
      loop {
        if (*v).next_sect.is_null() {
          v = (*v).next;
        } else {
          s = (*v).next_sect;
          loop {
            if (*s).valid {
              decomp_remain_sects(&mut s, &mut i, &mut new_data, &mut simple_cycles);
            }
            if (*(*s).dual).next.is_null() {
              v = (*(*s).other).next;
              break;
            } else {
              s = (*(*s).dual).next;
            }
          }
        }
        if (*v).i==vi {
          break;
        }   
      }
    }
    // -----

    (new_data, simple_cycles)
  }
}

// decomp check for remaining Sect
fn decomp_remain_sects(s: &mut *mut Sect, i: &mut usize, new_data: &mut Vec<f64>, simple_cycles: &mut Vec<SimpleCycle>) {
  unsafe {
    let mut s = *s;
    let si = (*s).i;
    let mut local_data: Vec<f64> = Vec::new();
    let mut success = true;

    loop {
      local_data.push((*s).x);
      local_data.push((*s).y);
      (*s).valid = false;

      if (*s).next.is_null() {
        success = false;
        break;
      } else {
        s = (*(*s).next).dual;
      }
      if (*(*s).dual).i==si {
        break;
      }
    }

    if success {
      let len: usize = local_data.len();
      let mut last: *mut Point;
      // check winding first;
      match signed_area(&local_data, 2) {
        Winding::Zero => {},
        Winding::CCW => { if (*s).sign {
          last = ptr::null_mut();
          for e in (0..len).step_by(2) {
            last = Point::new(*i, local_data[e], local_data[e+1], last);
            *i += 1;
          }
          new_data.extend(local_data);
          simple_cycles.push(SimpleCycle{ point: last, len: len/2 });
        } },
        Winding::CW => { if !(*s).sign {
          let mut new_local_data: Vec<f64> = Vec::new();
          last = ptr::null_mut();
          for e in (0..len).step_by(2).rev() {
            last = Point::new(*i, local_data[e], local_data[e+1], last);
            *i += 1;
            new_local_data.push(local_data[e]);
            new_local_data.push(local_data[e+1]);
          }
          new_data.extend(new_local_data);
          simple_cycles.push(SimpleCycle{ point: last, len: len/2 });
        } },
      }     
    }
  }
}



fn top_turn(v: &*mut Vertex) -> bool {
  unsafe {
    let mut v_prev = (*(*v)).prev;
    let mut v_next = (*(*v)).next;
  
    while (&(*(*v))).equals(&*v_prev) {
      v_prev = (*v_prev).prev;
      if (*(*v)).i == (*v_prev).i {
        break;
      }
    }
    while (&(*(*v))).equals(&*v_next) {
      v_next = (*v_next).next;
      if ((*(*v)).i==(*v_next).i) | ((*v_next).i==(*v_prev).i) {
        break;
      }
    }
  
    match area((*v_prev).x, (*v_prev).y, (*(*v)).x, (*(*v)).y, (*v_next).x, (*v_next).y) {
      Winding::CCW | Winding::Zero => true,
      Winding::CW => false,
    }
  }
}

pub fn update_sects(v: &*mut Vertex) {
  unsafe {
    // 1) get top vertex's turn
    let mut v: *mut Vertex = *v;
    let mut sign: bool = top_turn(&v);
    let vi = (*v).i;

    loop {
      (*v).sign = sign; // assign sign for each vertex;
      match &mut (*v).sects {
        None => {},
        Some(sects) => {
          // 2) sort Vertex.sects;
          if (*v).topdown { // in descending order
            sects.sort_by(|b, a| (&(*(*a))).partial_cmp(&(*(*b))).unwrap());
          } else { // in ascending order
            sects.sort_by(|a, b| (&(*(*a))).partial_cmp(&(*(*b))).unwrap());
          }

          // 3) restruct them to handle redundants;
          // (1) re-gather by uniqueness
          let mut resects: Vec<Vec<*mut Sect>> = Vec::new();
          let mut s_fmr = sects[0];
          resects.push(vec![s_fmr]);
          let mut s_now: *mut Sect;
          for i in 1..(sects.len()) {
            s_now = sects[i];
            if (&(*s_fmr)).eq(&(*s_now)) {
              let l = resects.len()-1;
              resects[l].push(s_now);
              s_fmr = s_now;
            } else {
              resects.push(vec![s_now]);
              s_fmr = s_now;
            }
          }

          // (2) select a path among redundants/and uniqueness
          let mut link_sects: Vec<Vec<*mut Sect>> = Vec::new(); // 중복점일 경우, 반드시 next 링크를 해줘야함(duality 고려) | For redundant points, you must make them linked next (for duality).
          for ss in resects.iter_mut() {
            if ss.len()==1 {
              // non redundancy
              link_sects.push(vec![ss[0]]);
              sign = !sign; // update sign
              (*(ss[0])).sign = sign;
            } else {
              // Yes redundancy!
              // make Vec<RedunSect> and sort it.
              let mut redunsects: Vec<RedunSect> = Vec::new();
              for (e, s) in ss.iter().enumerate() {
                let (r1, r2) = RedunSect::new(e, (*v).x, (*v).y, (*(*s)).x, (*(*s)).y, (*(*(*(*s)).other).next).x, (*(*(*(*s)).other).next).y);
                redunsects.push(r1);
                redunsects.push(r2);
              }
              redunsects.push(RedunSect{ i:0, dir: true, angle: std::f64::consts::FRAC_PI_2, is_straight: true}); // Key segment 방향도 고려해야 함. | Consider the direction of the key segment. 
              
              // sort from smaller to larger;
              if sign {
                redunsects.sort_by(|a, b| (&a).partial_cmp(&b).unwrap()); // in ascending order.
              } else {
                redunsects.sort_by(|b, a| (&a).partial_cmp(&b).unwrap()); // in descending order.
              }

              // find the path.   
              let mut key = 0;
              let mut r = &redunsects[0];
              for i in 0..(redunsects.len()) {
                r = &redunsects[i];
                key += if r.dir {1} else {-1};
                if key==1 {
                  break;
                }
              }

              if ss.len()%2==1 {
                sign = !sign; // update sign;
              }
              if ! r.is_straight { // 자기 자신으로 이동하는 path의 Sect는 연결하지 않음. | Do not link the Sect if it grows straight from the key segment.
                let mut ss_ = vec![ss[r.i]];
                (*(ss[r.i])).sign = sign;
                for s in ss.iter() {
                  if (*(*(*s)).other).i != (*(*ss[r.i]).other).i {
                    (*(*s)).sign = sign;
                    ss_.push(*s);
                  }
                }
                link_sects.push(ss_);
              }
            }
          }

          // (3) link the availables; and link v to the first sect;
          if link_sects.len()>0 {
            let mut ss0 = &link_sects[0];
            (*v).next_sect = ss0[0];

            let mut ss1: &Vec<*mut Sect>;
            for i in 1..(link_sects.len()) {
              ss1 = &link_sects[i];

              for s0 in ss0.iter() {
                (*(*s0)).next = ss1[0]; // next 연결은 하나로만. | Link next to the one.
              }
              ss0 = ss1;
            }
          }
        }
      }

      v = (*v).next;
      if (*v).i==vi {
        break;
      }
    }
  }
}

// ----- step 2. ----- //
pub fn update_intersect(array: &Vec<*mut Vertex>) -> bool {

  let len = array.len();
  let mut count: usize = 0;
  
  unsafe {
    let (mut v0, mut v1): (*mut Vertex, *mut Vertex);
    for i in 0..(len-1) {
      v0 = array[i];
      for j in (i+1)..len {
        v1 = array[j];
  
        if !(&*v0).is_adjacent(&*v1, len) {
          // don't need to check afterward.
          if (*v0).bottom > (*v1).top {
            break;
          }
          // bbox check;
          if ((*v0).left<=(*v1).right) & ((*v0).right>=(*v1).left) {
            // do intersect check
            if let Some((px, py, t, u)) = intersect(
              (*v0).x, (*v0).y, (*(*v0).next).x, (*(*v0).next).y,
              (*v1).x, (*v1).y, (*(*v1).next).x, (*(*v1).next).y,
            ) {
              // --
              if t==0. {
                let mut v0prev = (*v0).prev;
                let v0_nexti = (*(*v0).next).i;
                let (mut v0_0x, mut v0_0y) = ((*v0prev).x, (*v0prev).y);
                let (v0_1x, v0_1y) = ((*(*v0).next).x, (*(*v0).next).y);
                let (v1_0x, v1_0y) = ((*v1).x, (*v1).y);
                let (v1_1x, v1_1y) = ((*(*v1).next).x, (*(*v1).next).y);
                let mut area1 = area(v0_0x,v0_0y, px,py, v1_0x,v1_0y);
                while let Winding::Zero = area1{
                  v0prev = (*v0prev).prev;
                  if (*v0prev).i == v0_nexti {
                    break;
                  }
                  (v0_0x, v0_0y) = ((*v0prev).x, (*v0prev).y);
                  area1 = area(v0_0x,v0_0y, px,py, v1_0x,v1_0y);
                }
                if (*v0prev).i != v0_nexti {
                  if area1 == area(v0_1x,v0_1y, px,py, v1_1x,v1_1y) {
                    insert_sect(v0, v1, px, py, len+count);
                    count += 1;
                  }
                }
              } else if u==0. {
                let mut v1prev = (*v1).prev;
                let v1_nexti = (*(*v1).next).i;
                let (v0_0x, v0_0y) = ((*v0).x, (*v0).y);
                let (v0_1x, v0_1y) = ((*(*v0).next).x, (*(*v0).next).y);
                let (mut v1_0x, mut v1_0y) = ((*v1prev).x, (*v1prev).y);
                let (v1_1x, v1_1y) = ((*(*v1).next).x, (*(*v1).next).y);
                let mut area1 = area(v0_0x,v0_0y, px,py, v1_0x,v1_0y);
                while let Winding::Zero = area1 {
                  v1prev = (*v1prev).prev;
                  if (*v1prev).i == v1_nexti {
                    break;
                  }
                  (v1_0x, v1_0y) = ((*v1prev).x, (*v1prev).y);
                  area1 = area(v0_0x,v0_0y, px,py, v1_0x,v1_0y);
                }
                if (*v1prev).i != v1_nexti {
                  if area1 == area(v0_1x,v0_1y, px,py, v1_1x,v1_1y) {
                    insert_sect(v0, v1, px, py, len+count);
                    count += 1;
                  }
                }
              } else {
                insert_sect(v0, v1, px, py, len+count);
                count += 1;
              }
            }
          }
        }  
      }
    }
  }
  if count>0 { true } else { false }
}

fn intersect(x1:f64,y1:f64, x2:f64,y2:f64, x3:f64,y3:f64, x4:f64,y4:f64)
 -> Option<(f64, f64, f64, f64)> {

  let denominator = (x1-x2)*(y3-y4)-(y1-y2)*(x3-x4);
  if denominator==0. { // Don't care collinear cases.
    return None;
  }

  let mut t = (x1-x3)*(y3-y4)-(y1-y3)*(x3-x4);
  t /= denominator;
  if !((0.<=t)&(t<1.)) {// We don't consider intersections at ending-endpoints ;
    return None;
  }

  let mut u = (x1-x3)*(y1-y2)-(y1-y3)*(x1-x2);
  u /= denominator;
  if !((0.<=u)&(u<1.)) {
    return None;
  }

  let px = x1 + t * (x2-x1);
  let py = y1 + t * (y2-y1);
  return Some((px, py, t, u));
}

fn insert_sect<'a>(v0: *mut Vertex<'a>, v1: *mut Vertex<'a>, px:f64, py:f64, i: usize) {
  let mut sect1 = Box::into_raw(Box::new(Sect { i: i, x: px, y: py, dual: ptr::null_mut(), next: ptr::null_mut(), other: v1, sign: true, valid: true }));
  let mut sect2 = Box::into_raw(Box::new(Sect { i: i, x: px, y: py, dual: ptr::null_mut(), next: ptr::null_mut(), other: v0, sign: true, valid: true }));
  unsafe {
    (*sect1).dual = sect2;
    (*sect2).dual = sect1;

    match (*v0).sects {
      None => { (*v0).sects = Some(vec![sect1]); },
      _ => { (*v0).sects.as_mut().unwrap().push(sect1); }
    };
    match (*v1).sects {
      None => { (*v1).sects = Some(vec![sect2]); },
      _ => { (*v1).sects.as_mut().unwrap().push(sect2); }
    };
  }
}

// ----- step 1. ----- //
pub fn linked_vertex_array(data: &mut Vec<f64>, dim: usize) -> Vec<*mut Vertex> {

  // Make CCW winding linked array.
  match signed_area(data, dim) {
    Winding::CCW | Winding::Zero => fill_linked_vertex_array(true, data, dim),
    Winding::CW => fill_linked_vertex_array(false, data, dim),
  }
}

fn fill_linked_vertex_array(order: bool, data: &mut Vec<f64>, dim: usize) -> Vec<*mut Vertex> {
  // make sure the length is devided by the dim.
  let mut len = data.len();
  while len%dim>0 {
    data.remove(len-1);
    len = data.len();
  }

  // If last coord equals first coord, delete it.
  // Make sure first and last index are differnet. (to make sure two vertices with same xy coords can do partial_cmp with smaller-i priority)
  while len>=dim {
    if data[0]==data[len-dim] && data[1]==data[len-dim+1] {
      for i in ((len-dim)..len).rev() {
        data.remove(i);
      }
      len = data.len()
    } else {
      break;
    }
  }

  // Make linked Vertices while update their bbox & topdown; also push them into a vec(let array);
  /* true order: [a,b, c,d, e,f] => [(a,b), (c,d), (e,f)]
     false order: [a,b, c,d, e,f] => [(e,f), (c,d), (a,b)]
   */
  let mut array: Vec<*mut Vertex> = Vec::new();
  if len>dim {
    let mut x0: f64; let mut y0: f64; let mut x1: f64; let mut y1: f64;
    let mut last = ptr::null_mut();
    match order {
      true => {
        x0 = data[0]; y0 = data[1];
        for (e, i) in (dim..len).step_by(dim).enumerate() {
          x1 = data[i]; y1 = data[i+1];
          last = Vertex::new(e, x0, y0, x1, y1, last);
          array.push(last);
          x0 = x1; y0 = y1;
        }
        x1 = data[0]; y1 = data[1];
        last = Vertex::new(len/dim-1, x0, y0, x1, y1, last);
        array.push(last);
      },
      false => {
        x0 = data[len-dim]; y0 = data[len-dim+1];
        for (e, i) in (0..len-dim).step_by(dim).rev().enumerate() {
          x1 = data[i]; y1 = data[i+1];
          last = Vertex::new(e, x0, y0, x1, y1, last);
          array.push(last);
          x0 = x1; y0 = y1;
        }
        x1 = data[len-dim]; y1 = data[len-dim+1]; 
        last = Vertex::new(len/dim-1, x0, y0, x1, y1, last);
        array.push(last);
      },
    }
  }
  array
}



// ----------- //
// -- TESTs -- //
// ----------- //

mod test {
  #[test]
  fn test_signed_area() {
    use super::signed_area;
    use super::Winding;
    let data: Vec<f64> = vec![0.,0., 2.,0., 1.,1.];
    let result = signed_area(&data, 2);
    assert_eq!(result, Winding::CCW);

    let data: Vec<f64> = vec![0.,0., 1.,1., 2.,0.];
    let result = signed_area(&data, 2);
    assert_eq!(result, Winding::CW);

    let data: Vec<f64> = vec![0.,0., 2.,2., 1.,1.];
    let result = signed_area(&data, 2);
    assert_eq!(result, Winding::Zero);
  }

  #[test]
  fn test_area() {
    use super::area;
    use super::Winding;
    let result = area(0.,0., 2.,0., 1.,1.);
    assert_eq!(result, Winding::CCW);

    let result = area(0.,0., 1.,1., 2.,0.);
    assert_eq!(result, Winding::CW);

    let result = area(0.,0., 2.,2., 1.,1.);
    assert_eq!(result, Winding::Zero);
  }

  #[test]
  fn test_linked_vertex_array() {
    use super::linked_vertex_array;
    use super::Vertex;
    fn do_test(data: &mut Vec<f64>, dim: usize) -> (Vec<f64>, Vec<usize>) {
      let mut array: Vec<*mut Vertex> = linked_vertex_array(data, dim);
      let mut v1 = Vec::new(); // push floats;
      let mut v2 = Vec::new(); // push indices;
      unsafe {
        for a in array.iter_mut() {
          v1.push((*(*a)).x.clone());
          v1.push((*(*a)).y.clone());
          v2.push((*(*a)).i.clone());
          drop(Box::from_raw(*a)); // consume
        }
      }
      return (v1, v2);
    }
    let mut data = vec![0.,0., 1.,1., 2.,0., 0.,0., 0.,0.];
    let (v1, v2) = do_test(&mut data, 2);
    assert_eq!(v1, vec![2.,0., 1.,1., 0.,0.]);
    assert_eq!(v2, vec![0, 1, 2]);

    let mut data = vec![0.,0., 2.,0., 1.,1., 0.,0., 0.,0.];
    let (v1, v2) = do_test(&mut data, 2);
    assert_eq!(v1, vec![0.,0., 2.,0., 1.,1.]);
    assert_eq!(v2, vec![0, 1, 2]);

    let mut data = vec![0.,0., 1.,1.];
    let (v1, v2) = do_test(&mut data, 2);
    assert_eq!(v1, vec![0.,0., 1.,1.]);
    assert_eq!(v2, vec![0, 1,]);

    let mut data = vec![0.,0., 0.,0.];
    let (v1, v2) = do_test(&mut data, 2);
    assert_eq!(v1, vec![]);
    assert_eq!(v2, vec![]);

    let mut data = vec![0.,0.];
    let (v1, v2) = do_test(&mut data, 2);
    assert_eq!(v1, vec![]);
    assert_eq!(v2, vec![]);
  }

  #[test]
  fn test_sort_by() {
    use super::linked_vertex_array;
    use super::Vertex;
    fn do_test(data: &mut Vec<f64>) -> (Vec<f64>, Vec<usize>) {
      let mut array = linked_vertex_array(data, 2);
      let mut v1: Vec<f64> = Vec::new();
      let mut v2: Vec<usize> = Vec::new();
      unsafe {
        array.sort_by(|b, a| (&(*(*a))).partial_cmp(&(*(*b))).unwrap());
        for a in array.iter() {
          v1.push((*(*a)).x);
          v1.push((*(*a)).y);
          v2.push((*(*a)).i);
          drop(Box::from_raw(*a));
        }
        return (v1, v2)
      }
    }
    let mut data = vec![0.,0., 2.,0., 2.,0., 2.,2., 0.,2.,];
    let (v1, v2) = do_test(&mut data);
    assert_eq!(v1, vec![0.,2., 2.,2., 0.,0., 2.,0., 2.,0.,]);
    assert_eq!(v2, vec![4, 3, 0, 1, 2]);
  }

  #[test]
  fn test_intersect() {
    use super::intersect;
    assert_eq!(Some((1.,0.,0.5,0.5)), intersect(0.,0., 2.,0., 1.,-1., 1.,1.));
  }

  #[test]
  fn test_top_turn() {
    use super::{linked_vertex_array, top_turn};

    fn do_test(data: &mut Vec<f64>) -> bool {
      let mut array = linked_vertex_array(data, 2);
      unsafe { array.sort_by(|b, a| (&(*(*a))).partial_cmp(&(*(*b))).unwrap()); }
      let sign = top_turn(&(array[0]));
      // consume raw pointers
      unsafe {
        for a in array.iter() {
          if let Some(sects) = &(*(*a)).sects {
            for s in sects.iter() {
              drop(Box::from_raw(*s));
            }
          }
          drop(Box::from_raw(*a));
        }
      }
      sign
    }

    let mut data = vec![-1.,0., -1.,-1., 1.,1., 1.,0.];
    let sign = do_test(&mut data);
    assert_eq!(sign, false);
    
    let mut data = vec![1.,1., 1.,1., -1.,1., -1.,1., -1.,1., 0.,0., 1.,0.];
    let sign = do_test(&mut data);
    assert_eq!(sign, true);

    let mut data = vec![1.,1., 1.,1., -1.,1., -1.,1., -2.,1., -1.,1., 0.,0., 1.,0.];
    let sign = do_test(&mut data);
    assert_eq!(sign, true);

    let mut data = vec![-2.,1., -2.,1., -1.,1., -1.,1., -1.,1., 1.,1., 1.,0., 0.,0.,];
    let sign = do_test(&mut data);
    assert_eq!(sign, true);

    let mut data = vec![-1.,1., -1.,1., -1.,1., 1.,1., 1.,1., -2.,-2., 2.,-2.,];
    let sign = do_test(&mut data);
    assert_eq!(sign, false);

    let mut data = vec![-1.,1., -1.,1., -1.,1., 1.,1.,];
    let sign = do_test(&mut data);
    assert_eq!(sign, true);
  }

  #[test]
  
  fn test_update_sects() {
    use super::{linked_vertex_array, update_intersect, update_sects, Vertex};
    unsafe {
      let mut data = vec![-1.,0., -1.,-1., 1.,1., 1.,0.];
      let mut array = linked_vertex_array(&mut data, 2);
      array.sort_by(|b, a| (&(*(*a))).partial_cmp(&(*(*b))).unwrap());
      if update_intersect(&mut array) {
        update_sects(&mut array[0]);

        for a in array.iter() {
          if (*(*a)).x==-1. && (*(*a)).y==-1. {
            if let Some(sects) = &(*(*a)).sects {
              assert_eq!(sects.len(), 1);
              // assert_eq!(sects.len(), 2); // Must be an error;
            }
          }
        }
      }
      unsafe {
        for a in array.iter() {
          if let Some(sects) = &(*(*a)).sects {
            for s in sects.iter() {
              drop(Box::from_raw(*s));
            }
          }
          drop(Box::from_raw(*a));
        }
      }

      let mut data = vec![-2.,-2., 2.,2., 1.,-2., -1.,2., 1.,2., -1.,-2., -2.,1., 2.,-1., 2.,0., -2.,0.,]; // 반대방향으로 연결됨.
      let mut array = linked_vertex_array(&mut data, 2);
      array.sort_by(|b, a| (&(*(*a))).partial_cmp(&(*(*b))).unwrap());
      if update_intersect(&mut array) {
        update_sects(&mut array[0]);

        for a in array.iter() {
          if (*(*a)).x==2. && (*(*a)).y==2. {
            if let Some(sects) = &(*(*a)).sects {
              //assert_eq!(sects.len(), 5);
              // assert_eq!(sects.len(), 2); // Must be an error;

              let mut t = 0;
              for s in sects.iter() {
                if ! (*(*s)).next.is_null() { t+=1; }
              }
              // assert_eq!(t, 1);
              // assert_eq!(i, 2); // Must be an error;
            }
            if ! (*(*a)).next_sect.is_null() {
              let x = (*(*(*(*a)).next_sect).other).x;
              let y = (*(*(*(*a)).next_sect).other).y;
              assert_eq!(x, -2.);
              assert_eq!(y, 0.);
            }
          }
        }
      }
      unsafe {
        for a in array.iter() {
          if let Some(sects) = &(*(*a)).sects {
            for s in sects.iter() {
              drop(Box::from_raw(*s));
            }
          }
          drop(Box::from_raw(*a));
        }
      }
    }
  }
  


  #[test]
  fn test_triangulate() {
    use super::triangulate;
    let mut data = vec![0.,0., 2.,0., 2.,0., 2.,2., 0.,2.,];
    let mut data = vec![-1.,0., -1.,-1., 1.,1., 1.,0.];
    let mut data = vec![-2.,-2., 2.,2., 1.,-2., -1.,2., 1.,2., -1.,-2., -2.,1., 2.,-1., 2.,0., -2.,0.,];
    let result = triangulate(&mut data, 2);
    // assert_eq!(result, (vec![1.0, 2.0], vec![1, 2])); // Would fail.
  }
}
