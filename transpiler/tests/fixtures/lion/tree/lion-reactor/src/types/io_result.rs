use vstd::prelude::*;
use crate::spec::types::IoResultView;

verus! {

#[verifier::external_body]
pub struct IoError {
  pub(crate) inner: std::io::Error,
}

impl View for IoError {
  type V = int;

  #[verifier::external_body]
  spec fn view(&self) -> int {
    unimplemented!()
  }
}

pub enum IoResult<T> {
  Ok(T),
  Err(IoError),
}

impl<T: View> View for IoResult<T> {
  type V = IoResultView<T::V>;

  open spec fn view(&self) -> IoResultView<T::V> {
    match self {
      IoResult::Ok(t) => IoResultView::Ok(t@),
      IoResult::Err(e) => IoResultView::Err(e@),
    }
  }
}

impl<T: DeepView> DeepView for IoResult<T> {
  type V = IoResultView<T::V>;

  open spec fn deep_view(&self) -> IoResultView<T::V> {
    match self {
      IoResult::Ok(t) => IoResultView::Ok(t.deep_view()),
      IoResult::Err(e) => IoResultView::Err(e@),
    }
  }
}

impl IoError {
  #[verifier::external_body]
  pub fn resource_id_overflow() -> Self {
    IoError {
      inner: std::io::Error::new(
        std::io::ErrorKind::Other,
        "reactor resource ID overflow: maximum resource IDs exhausted"
      ),
    }
  }
}

} // end verus!

impl Clone for IoError {
  fn clone(&self) -> Self {
    IoError {
      inner: std::io::Error::new(self.inner.kind(), self.inner.to_string()),
    }
  }
}

impl IoError {
  /// The underlying `std::io::Error`.
  pub fn into_io_error(self) -> std::io::Error {
    self.inner
  }
}

impl From<IoError> for std::io::Error {
  fn from(e: IoError) -> std::io::Error {
    e.inner
  }
}

impl std::fmt::Debug for IoError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.inner.fmt(f)
  }
}
