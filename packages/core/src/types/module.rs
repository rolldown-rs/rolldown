pub enum RollDownModule {
  Normal,
  External,
}

impl RollDownModule {
  pub fn is_normal(&self) -> bool {
    if let RollDownModule::Normal = self {
      true
    } else {
      false
    }
  }
  pub fn is_external(&self) -> bool {
    !self.is_normal()
  }
}
