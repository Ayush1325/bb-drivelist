use std::process::Command;

use serde::Deserialize;

use crate::device::{DeviceDescriptor, MountPoint};

#[derive(Deserialize, Debug)]
struct Disks {
    #[serde(rename = "AllDisksAndPartitions")]
    all_disks_and_partitions: Vec<Disk>,
}

#[derive(Deserialize, Debug)]
struct Disk {
    #[serde(rename = "DeviceIdentifier")]
    device_identifier: String,
    #[serde(rename = "OSInternal")]
    os_internal: bool,
    #[serde(rename = "Size")]
    size: u64,
    #[serde(rename = "Content")]
    content: String,
    #[serde(rename = "Partitions", default)]
    partitions: Vec<Partition>,
}

#[derive(Deserialize, Debug)]
struct Partition {
    #[serde(rename = "MountPoint")]
    mount_point: Option<String>,
    #[serde(rename = "Content")]
    content: String,
    #[serde(rename = "Size")]
    size: u64,
}

#[derive(Deserialize, Debug)]
struct DiskInfo {
    #[serde(rename = "MediaName")]
    media_name: Option<String>,
}

fn get_media_name(device_identifier: &str) -> Option<String> {
    let output = Command::new("diskutil")
        .args(["info", "-plist", device_identifier])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let info: DiskInfo = plist::from_bytes(&output.stdout).ok()?;
    info.media_name
}

impl From<Disk> for DeviceDescriptor {
    fn from(value: Disk) -> Self {
        let description = if !value.content.is_empty() 
            && !value.content.contains("partition_scheme")
            && !value.content.contains("_Container") {
            value.content.clone()
        } else if let Some(name) = get_media_name(&value.device_identifier) {
            name
        } else {
            format!("{} GB Disk", value.size / 1_000_000_000)
        };

        DeviceDescriptor {
            enumerator: "diskutil".to_string(),
            description,
            size: value.size,
            mountpoints: value.partitions.into_iter().map(MountPoint::from).collect(),
            device: format!("/dev/{}", value.device_identifier),
            raw: format!("/dev/r{}", value.device_identifier),
            is_system: value.os_internal,
            is_removable: !value.os_internal,
            ..Default::default()
        }
    }
}

impl From<Partition> for MountPoint {
    fn from(value: Partition) -> Self {
        MountPoint {
            path: value.mount_point.unwrap_or_default(),
            label: Some(value.content),
            total_bytes: Some(value.size),
            available_bytes: None,
        }
    }
}

pub(crate) fn diskutil() -> anyhow::Result<Vec<DeviceDescriptor>> {
    let output = Command::new("diskutil").args(["list", "-plist"]).output()?;

    if !output.status.success() {
        return Err(anyhow::Error::msg("diskutil fail"));
    }

    let parsed: Disks = plist::from_bytes(&output.stdout).unwrap();

    Ok(parsed
        .all_disks_and_partitions
        .into_iter()
        .map(DeviceDescriptor::from)
        .collect())
}
