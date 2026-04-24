use std::cmp::Ordering;

use zed_extension_api::{self as zed, http_client, serde_json, Result};

const NUGET_FEED_INDEX: &str = "https://api.nuget.org/v3/index.json";

pub struct NuGetClient {
    package_base_address: Option<String>,
}

impl NuGetClient {
    pub fn new() -> Self {
        Self {
            package_base_address: None,
        }
    }

    fn ensure_package_base_address(&mut self) -> Result<String> {
        if let Some(base) = &self.package_base_address {
            return Ok(base.clone());
        }

        let response = http_client::fetch(
            &http_client::HttpRequest::builder()
                .method(http_client::HttpMethod::Get)
                .url(NUGET_FEED_INDEX)
                .redirect_policy(http_client::RedirectPolicy::FollowAll)
                .build()?,
        )?;

        let index: serde_json::Value = serde_json::from_slice(&response.body)
            .map_err(|e| format!("failed to parse NuGet service index: {e}"))?;

        let base_url = index["resources"]
            .as_array()
            .ok_or("invalid NuGet service index: missing 'resources' array")?
            .iter()
            .find(|resource| {
                resource["@type"]
                    .as_str()
                    .is_some_and(|kind| kind == "PackageBaseAddress/3.0.0")
            })
            .and_then(|resource| resource["@id"].as_str())
            .ok_or("PackageBaseAddress/3.0.0 not found in NuGet service index")?
            .trim_end_matches('/')
            .to_string();

        self.package_base_address = Some(base_url.clone());
        Ok(base_url)
    }

    pub fn get_latest_version(&mut self, package_id: &str) -> Result<String> {
        let base = self.ensure_package_base_address()?;
        let lower_id = package_id.to_lowercase();
        let url = format!("{base}/{lower_id}/index.json");

        let response = http_client::fetch(
            &http_client::HttpRequest::builder()
                .method(http_client::HttpMethod::Get)
                .url(&url)
                .redirect_policy(http_client::RedirectPolicy::FollowAll)
                .build()?,
        )?;

        let body: serde_json::Value = serde_json::from_slice(&response.body)
            .map_err(|e| format!("failed to parse NuGet version index for '{package_id}': {e}"))?;

        let versions = body["versions"]
            .as_array()
            .ok_or_else(|| format!("no versions array for NuGet package '{package_id}'"))?;

        versions
            .iter()
            .filter_map(|value| value.as_str())
            .filter_map(NuGetVersion::parse)
            .max()
            .map(|version| version.raw)
            .ok_or_else(|| format!("no parseable versions found for NuGet package '{package_id}'"))
    }

    pub fn download_and_extract(
        &mut self,
        package_id: &str,
        version: &str,
        dest_dir: &str,
    ) -> Result<()> {
        let base = self.ensure_package_base_address()?;
        let lower_id = package_id.to_lowercase();
        let lower_version = version.to_lowercase();
        let url = format!("{base}/{lower_id}/{lower_version}/{lower_id}.{lower_version}.nupkg");

        zed::download_file(&url, dest_dir, zed::DownloadedFileType::Zip)
            .map_err(|e| format!("failed to download NuGet package '{package_id}' v{version}: {e}"))
    }
}

#[derive(Debug, Clone)]
struct NuGetVersion {
    major: u64,
    minor: u64,
    patch: u64,
    revision: u64,
    prerelease: Option<String>,
    raw: String,
}

impl NuGetVersion {
    fn parse(input: &str) -> Option<Self> {
        let (core, prerelease) = match input.split_once('-') {
            Some((core, prerelease)) => (core, Some(prerelease.to_string())),
            None => (input, None),
        };

        let segments = core
            .split('.')
            .map(|segment| segment.parse::<u64>().ok())
            .collect::<Option<Vec<_>>>()?;

        let (major, minor, patch, revision) = match segments.as_slice() {
            [major] => (*major, 0, 0, 0),
            [major, minor] => (*major, *minor, 0, 0),
            [major, minor, patch] => (*major, *minor, *patch, 0),
            [major, minor, patch, revision] => (*major, *minor, *patch, *revision),
            _ => return None,
        };

        Some(Self {
            major,
            minor,
            patch,
            revision,
            prerelease,
            raw: input.to_string(),
        })
    }
}

impl PartialEq for NuGetVersion {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for NuGetVersion {}

fn cmp_prerelease_token(left: &str, right: &str) -> Ordering {
    match (left.parse::<u64>(), right.parse::<u64>()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        (Ok(_), Err(_)) => Ordering::Less,
        (Err(_), Ok(_)) => Ordering::Greater,
        (Err(_), Err(_)) => left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase()),
    }
}

impl Ord for NuGetVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
            .then(self.revision.cmp(&other.revision))
            .then_with(|| match (&self.prerelease, &other.prerelease) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(left), Some(right)) => {
                    let mut left_parts = left.split('.');
                    let mut right_parts = right.split('.');
                    loop {
                        match (left_parts.next(), right_parts.next()) {
                            (Some(left), Some(right)) => {
                                let ordering = cmp_prerelease_token(left, right);
                                if ordering != Ordering::Equal {
                                    return ordering;
                                }
                            }
                            (None, Some(_)) => return Ordering::Less,
                            (Some(_), None) => return Ordering::Greater,
                            (None, None) => return Ordering::Equal,
                        }
                    }
                }
            })
    }
}

impl PartialOrd for NuGetVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
