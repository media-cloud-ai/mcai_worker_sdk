
#[derive(Debug)]
pub struct WorkerConfiguration {
  identifier: String,
  version: Version,
}

impl WorkerConfiguration {
  pub fn new(identifier: &str, version: Version) -> Self {
    WorkerConfiguration {
      identifier: identifier.to_string(),
      version
    }
  }
}

#[derive(Debug)]
pub struct Version {
  major: u32,
  minor: u32,
  patch: u32,
  pre_release: Vec<String>,
  build: Vec<String>,
}

impl Version {
  pub fn new(major: u32, minor: u32, patch: u32) -> Self {
    Version {
      major,
      minor,
      patch,
      pre_release: vec![],
      build: vec![],
    }
  }

  pub fn new_with_pre_release(major: u32, minor: u32, patch: u32, pre_release: Vec<String>) -> Self {
    Version {
      major,
      minor,
      patch,
      pre_release,
      build: vec![],
    }
  }

  pub fn new_with_build(major: u32, minor: u32, patch: u32, build: Vec<String>) -> Self {
    Version {
      major,
      minor,
      patch,
      pre_release: vec![],
      build,
    }
  }

  pub fn new_with_pre_release_and_build(major: u32, minor: u32, patch: u32, pre_release: Vec<String>, build: Vec<String>) -> Self {
    Version {
      major,
      minor,
      patch,
      pre_release,
      build,
    }
  }
}

impl ToString for Version {
  fn to_string(&self) -> String {
    let pre_release =
      if self.pre_release.is_empty() {
        "".to_string()
      } else {
        format!("-{}", self.pre_release.join(","))
      };

    let extension =
      if self.build.is_empty() {
        pre_release
      } else {
        format!("{}+{}", pre_release, self.build.join(","))
      };

    format!("{}.{}.{}{}", self.major, self.minor, self.patch, extension)
  }
}

#[test]
fn version_to_string() {
  let v = Version::new(1, 2, 3);
  assert_eq!(v.to_string(), "1.2.3");

  let v = Version::new_with_pre_release(1, 2, 3, vec!["rc1".to_string()]);
  assert_eq!(v.to_string(), "1.2.3-rc1");

  let v = Version::new_with_build(1, 2, 3, vec!["ac6dv2".to_string()]);
  assert_eq!(v.to_string(), "1.2.3+ac6dv2");

  let v = Version::new_with_pre_release_and_build(1, 2, 3, vec!["rc1".to_string()], vec!["ac6dv2".to_string()]);
  assert_eq!(v.to_string(), "1.2.3-rc1+ac6dv2");

}
