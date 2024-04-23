//! # Robust Triangulation with Louvre 🌙
//! 
//! Louvre is a robust triangulation algorithm, which can handle self-intersecting polygons' triangulation.
//! 
//! What is triangulation? ["Polygon triangulation"](https://en.wikipedia.org/wiki/Polygon_triangulation)
//! makes a polygon's vertex coordinates array
//! into a set of coordinates of triangles, whose areas' sum equals to the polygon's.
//! 
//! As most of computational graphic processing systems (like opengl/webgl)
//! handle polygons by decomposing them into triangles,
//! a good and fast triangulation algorithm is crucial.
//! 
//! Earcut([mapbox/earcut.js](https://github.com/mapbox/earcut)) is one of the most widely used triangulation algorithm.
//! However simple earcut cannot properly decompose self-intersecting polygons.
//! 
//! Louvre widely refered to mapbox/earcut.js to implement basic logics and utilities of simple earcut algorithm.
//! Making further contribution,
//! louvre can handle **self-intersecting polygons**, which is not viable in most open source algorithms including mapbox/earcut.js.
//! 
//! See [the live demo](https://acheul.github.io/louvre) of louvre. Try drawing some complex polygons, with self-intersecting lines.
//! 
//! Surely, louvre is FAST and ROBUST.
//! You can use this in rust native or rust supporting wasm environment.
//! 
//! 
//! # Ex
//! ```rust
//! use louvre::triangulate;
//! 
//! let mut data: Vec<f64> = vec![
//!   [0., 0.], [0., 3.], [3., 0.], [3., 4.], [-1., 0.]
//! ].concat();
//! 
//! let (new_data, indices) = triangulate(&mut data, 2);
//! 
//! assert_eq!(new_data, 
//!   vec![
//!     3.0, 0.0,  3.0, 4.0,  1.0, 2.0, 
//!     0.0, 0.0,  0.0, 1.0,  -1.0, 0.0,
//!     0.0, 1.0,  1.0, 2.0,  0.0, 3.0
//!   ]
//! );
//! assert_eq!(indices, vec![
//!   1, 2, 0, 
//!   4, 5, 3, 
//!   7, 8, 6
//! ]);
//! ```


pub mod triangulate;
pub use triangulate::*;
pub use triangulate::triangulate;

pub mod utils;
use utils::*;

pub mod structures;
use structures::*;

use std::cmp::Ordering;
use std::ptr;
use std::f64;

#[cfg(feature="html")]
pub mod html;