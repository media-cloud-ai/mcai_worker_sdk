use std::fs;
use uuid::Uuid;

static mut INSTANCE_UUID: Option<String> = None;

/// Retrieve the identifier of this instance.
///
/// It can be the Docker container ID.
/// Else an UUID is generated to provide an unique identifier.
///
pub fn get_instance_id(filename: &str) -> String {
  fs::read_to_string(filename)
    .map(|content| parse_docker_container_id(&content))
    .unwrap_or_else(|_| None)
    .unwrap_or_else(|| unsafe {
      if let Some(instance_uuid) = &INSTANCE_UUID {
        instance_uuid.to_string()
      } else {
        let identifier = format!("{:?}", Uuid::new_v4());
        INSTANCE_UUID = Some(identifier.clone());
        identifier
      }
    })
}

fn parse_docker_container_id(content: &str) -> Option<String> {
  let lines: Vec<&str> = content.split('\n').collect();
  if lines.is_empty() {
    return None;
  }
  let items: Vec<&str> = lines[0].split(':').collect();
  if items.len() != 3 {
    return None;
  }

  let long_identifier: Vec<&str> = items[2].split("/docker/").collect();
  if long_identifier.len() != 2 {
    return None;
  }
  let mut identifier = long_identifier[1].to_string();
  identifier.truncate(12);
  Some(identifier)
}

#[test]
fn test_get_instance_id() {
  assert_eq!(
    get_instance_id("./tests/cgroup.sample"),
    "da9002cb1553".to_string()
  );

  let str_uuid = get_instance_id("/tmp/file_not_exists");
  let parsed_uuid = Uuid::parse_str(&str_uuid);
  assert!(parsed_uuid.is_ok());

  assert_eq!(parse_docker_container_id(""), None);
  assert_eq!(parse_docker_container_id("\n"), None);
  assert_eq!(parse_docker_container_id("a:b:c\n"), None);
}
