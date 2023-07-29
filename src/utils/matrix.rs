use core::mem;
use core::fmt;
use core::cell::UnsafeCell;
use core::ops::{IndexMut, Index, Mul};

// Square matrix with interior mutability
pub struct SquareMatrix {
  size: usize,
  data: Box<[UnsafeCell<f32>]>
}
unsafe impl Sync for SquareMatrix {}

impl SquareMatrix {
  pub fn new(size: usize) -> SquareMatrix {
    let data: Vec<f32> = vec![0.0; size * size];
    SquareMatrix{
      size,
      // Safety: f32 and UnsafeCell<f32> have the same representation in memory
      data: unsafe { mem::transmute(data.into_boxed_slice()) }
    }
  }

  #[inline(always)]
  pub fn size(&self) -> usize {
    self.size
  }

  #[inline(always)]
  pub fn write(&self, index: (usize, usize), value: f32) {
    unsafe {
      *self.get_unsafe_cell(index).get() = value;
    }
  }

  // Row major
  #[inline(always)]
  fn linear_index(&self, (row, column): (usize, usize)) -> usize {
    assert!(row < self.size);
    assert!(column < self.size);
    row * self.size + column
  }

  #[inline(always)]
  pub fn get_unsafe_cell(&self, index: (usize, usize)) -> &UnsafeCell<f32> {
    unsafe {
      &self.data.get_unchecked(self.linear_index(index))
    }
  }

  pub fn upper_triangle_with_diagonal(&self) -> SquareMatrix {
    let mut output = SquareMatrix::new(self.size);

    for row in 0 .. self.size {
      for column in row .. self.size {
        output[(row, column)] = self[(row, column)];
      }
    }

    output
  }

  pub fn lower_triangle_with_1_diagonal(&self) -> SquareMatrix {
    let mut output = SquareMatrix::new(self.size);

    for row in 0 .. self.size {
      for column in 0 .. row {
        output[(row, column)] = self[(row, column)];
      }
    }

    for i in 0 .. self.size {
      output[(i, i)] = 1.0;
    }

    output
  }

  pub fn copy_to(&self, other: &SquareMatrix) {
    assert_eq!(self.data.len(), other.data.len());
    for i in 0 .. self.data.len() {
      unsafe {
        *(other.data[i].get()) = *self.data[i].get();
      }
    }
  }
}

impl Clone for SquareMatrix {
  fn clone(&self) -> Self {
    let mut data: Vec<f32> = vec![0.0; self.size * self.size];
    for i in 0 .. data.len() {
      data[i] = unsafe { *self.data[i].get() };
    }
    SquareMatrix { size: self.size, data: unsafe { mem::transmute(data.into_boxed_slice()) } }
  }
}

impl Index<(usize, usize)> for SquareMatrix {
  type Output = f32;

  #[inline(always)]
  fn index(&self, index: (usize, usize)) -> &Self::Output {
    unsafe { &*self.get_unsafe_cell(index).get() }
  }
}

impl IndexMut<(usize, usize)> for SquareMatrix {
  #[inline(always)]
  fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
    unsafe { &mut *self.get_unsafe_cell(index).get() }
  }
}

impl Mul for &SquareMatrix {
  type Output = SquareMatrix;

  fn mul(self, other: &SquareMatrix) -> Self::Output {
    assert_eq!(self.size, other.size);
    let mut output = SquareMatrix::new(self.size);
    for row in 0 .. self.size {
      for column in 0 .. self.size {
        let mut sum = 0.0;
        for k in 0 .. self.size {
          sum += self[(row, k)] * other[(k, column)];
        }
        output[(row, column)] = sum;
      }
    }
    output
  }
}

impl fmt::Debug for SquareMatrix {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "SquareMatrix ({}x{}) {{", self.size, self.size)?;
    for row in 0 .. self.size {
      write!(f, "\n ")?;
      for column in 0 .. self.size {
        write!(f, " {}", self[(row, column)])?;
      }
    }
    write!(f, "\n}}")
  }
}
