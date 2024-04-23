use louvre::*;
use louvre::utils::{Winding, signed_area, area};
use louvre::structures::*;

#[test]
fn test_signed_area() {
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
  let result = area(0.,0., 2.,0., 1.,1.);
  assert_eq!(result, Winding::CCW);

  let result = area(0.,0., 1.,1., 2.,0.);
  assert_eq!(result, Winding::CW);

  let result = area(0.,0., 2.,2., 1.,1.);
  assert_eq!(result, Winding::Zero);
}

#[test]
fn test_linked_vertex_array() {
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
  assert_eq!(v1, Vec::<f64>::new());
  assert_eq!(v2, Vec::<usize>::new());

  let mut data = vec![0.,0.];
  let (v1, v2) = do_test(&mut data, 2);
  assert_eq!(v1, Vec::<f64>::new());
  assert_eq!(v2, Vec::<usize>::new());
}

#[test]
fn test_sort_by() {
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
  assert_eq!(Some((1.,0.,0.5,0.5)), intersect(0.,0., 2.,0., 1.,-1., 1.,1.));
}

#[test]
fn test_top_turn() {

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
    for a in array.iter() {
      if let Some(sects) = &(*(*a)).sects {
        for s in sects.iter() {
          drop(Box::from_raw(*s));
        }
      }
      drop(Box::from_raw(*a));
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



#[test]
fn test_triangulate() {
  //let data = vec![0.,0., 2.,0., 2.,0., 2.,2., 0.,2.,];
  //let data = vec![-1.,0., -1.,-1., 1.,1., 1.,0.];
  //let mut data = vec![-2.,-2., 2.,2., 1.,-2., -1.,2., 1.,2., -1.,-2., -2.,1., 2.,-1., 2.,0., -2.,0.,];
  let mut data: Vec<f64> = vec![
    [0., 0.], [0., 3.], [3., 0.], [3., 4.], [-1., 0.]
  ].concat();
  let (new_data, indices) = triangulate(&mut data, 2);
  assert_eq!(new_data, 
    vec![
      3.0, 0.0, 3.0, 4.0, 1.0, 2.0, 
      0.0, 0.0, 0.0, 1.0, -1.0, 0.0,
      0.0, 1.0, 1.0, 2.0, 0.0, 3.0
    ]
  );
  assert_eq!(indices, vec![1, 2, 0, 4, 5, 3, 7, 8, 6]);
}