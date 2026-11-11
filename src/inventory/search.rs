use crate::core::models::Asset;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default)]
pub struct AssetFilters<'a> {
    pub text: Option<&'a str>,
    pub group: Option<&'a str>,
    pub asset_type: Option<&'a str>,
    pub application: Option<&'a str>,
    pub protocol: Option<&'a str>,
    pub service_label: Option<&'a str>,
    pub web_label: Option<&'a str>,
    pub port: Option<u16>,
    pub network: Option<&'a str>,
    pub location: Option<&'a str>,
    pub software_vendor: Option<&'a str>,
    pub software_product: Option<&'a str>,
    pub certificate_label: Option<&'a str>,
    pub issuer: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub struct FacetGroup {
    pub name: &'static str,
    pub values: Vec<Facet>,
}

#[derive(Debug, Serialize)]
pub struct Facet {
    pub value: String,
    pub count: usize,
}

pub fn filter_assets<'a>(assets: &'a [Asset], filters: &AssetFilters<'_>) -> Vec<&'a Asset> {
    assets
        .iter()
        .filter(|asset| {
            filters
                .group
                .is_none_or(|value| equals_ignore_ascii_case(&asset.group, value))
                && filters
                    .asset_type
                    .is_none_or(|value| equals_ignore_ascii_case(&asset.asset_type, value))
                && filters
                    .text
                    .is_none_or(|value| searchable_text(asset).contains(&value.to_lowercase()))
                && filters
                    .application
                    .is_none_or(|value| contains_value(&asset.applications, value))
                && filters
                    .protocol
                    .is_none_or(|value| contains_value(&asset.protocols, value))
                && filters
                    .service_label
                    .is_none_or(|value| contains_value(&asset.service_labels, value))
                && filters
                    .web_label
                    .is_none_or(|value| contains_value(&asset.web_labels, value))
                && filters
                    .port
                    .is_none_or(|value| asset.ports.contains(&value))
                && filters
                    .network
                    .is_none_or(|value| contains_value(&asset.networks, value))
                && filters
                    .location
                    .is_none_or(|value| contains_value(&asset.locations, value))
                && filters
                    .software_vendor
                    .is_none_or(|value| contains_value(&asset.software_vendors, value))
                && filters
                    .software_product
                    .is_none_or(|value| contains_value(&asset.software_products, value))
                && filters
                    .certificate_label
                    .is_none_or(|value| contains_value(&asset.certificate_labels, value))
                && filters
                    .issuer
                    .is_none_or(|value| contains_value(&asset.issuers, value))
        })
        .collect()
}

pub fn facets(assets: &[&Asset]) -> Vec<FacetGroup> {
    vec![
        facet_group("ASSET TYPES", assets, |asset| {
            vec![asset.asset_type.clone()]
        }),
        facet_group("NETWORKS (AS)", assets, |asset| asset.networks.clone()),
        facet_group("LOCATIONS", assets, |asset| asset.locations.clone()),
        facet_group("APPLICATIONS", assets, |asset| asset.applications.clone()),
        facet_group("SERVICE LABELS", assets, |asset| {
            asset.service_labels.clone()
        }),
        facet_group("WEB LABELS", assets, |asset| asset.web_labels.clone()),
        facet_group("TRANSPORT PROTOCOLS", assets, |asset| {
            asset.protocols.clone()
        }),
        facet_group("PORTS", assets, |asset| {
            asset.ports.iter().map(u16::to_string).collect()
        }),
        facet_group("SOFTWARE VENDORS", assets, |asset| {
            asset.software_vendors.clone()
        }),
        facet_group("SOFTWARE PRODUCTS", assets, |asset| {
            asset.software_products.clone()
        }),
        facet_group("CERTIFICATE LABELS", assets, |asset| {
            asset.certificate_labels.clone()
        }),
        facet_group("ISSUERS", assets, |asset| asset.issuers.clone()),
    ]
}

fn facet_group(
    name: &'static str,
    assets: &[&Asset],
    values: impl Fn(&Asset) -> Vec<String>,
) -> FacetGroup {
    let mut counts = BTreeMap::<String, usize>::new();
    for asset in assets {
        let unique = values(asset).into_iter().collect::<BTreeSet<_>>();
        for value in unique {
            *counts.entry(value).or_default() += 1;
        }
    }
    let mut values = counts
        .into_iter()
        .map(|(value, count)| Facet { value, count })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.value.cmp(&right.value))
    });
    FacetGroup { name, values }
}

fn contains_value(values: &[String], filter: &str) -> bool {
    values
        .iter()
        .any(|value| value.to_lowercase().contains(&filter.to_lowercase()))
}

fn equals_ignore_ascii_case(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

fn searchable_text(asset: &Asset) -> String {
    let mut fields = vec![
        asset.host.as_str(),
        asset.label.as_str(),
        asset.group.as_str(),
    ];
    fields.extend(asset.ip_addresses.iter().map(String::as_str));
    fields.extend(asset.networks.iter().map(String::as_str));
    fields.extend(asset.locations.iter().map(String::as_str));
    fields.extend(asset.applications.iter().map(String::as_str));
    fields.extend(asset.protocols.iter().map(String::as_str));
    fields.extend(asset.service_labels.iter().map(String::as_str));
    fields.extend(asset.web_labels.iter().map(String::as_str));
    fields.extend(asset.software_vendors.iter().map(String::as_str));
    fields.extend(asset.software_products.iter().map(String::as_str));
    fields.extend(asset.certificate_labels.iter().map(String::as_str));
    fields.extend(asset.issuers.iter().map(String::as_str));
    fields.extend(asset.endpoints.iter().map(String::as_str));
    fields.extend(asset.matched_fields.values().map(String::as_str));
    fields.extend(asset.matched_fields.keys().map(String::as_str));
    fields.join(" ").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(host: &str, location: &str, ports: &[u16]) -> Asset {
        Asset {
            host: host.into(),
            asset_type: "host".into(),
            group: "production".into(),
            locations: vec![location.into()],
            ports: ports.to_vec(),
            applications: vec!["HTTP".into()],
            ..Asset::default()
        }
    }

    #[test]
    fn filters_assets_and_facets_count_unique_assets() {
        let assets = vec![
            sample("one.example.com", "Saudi Arabia", &[80, 443]),
            sample("two.example.com", "Saudi Arabia", &[443]),
            sample("three.example.com", "United States", &[22]),
        ];
        let filtered = filter_assets(
            &assets,
            &AssetFilters {
                location: Some("saudi"),
                port: Some(443),
                ..AssetFilters::default()
            },
        );
        assert_eq!(filtered.len(), 2);
        let groups = facets(&filter_assets(&assets, &AssetFilters::default()));
        let locations = groups
            .iter()
            .find(|group| group.name == "LOCATIONS")
            .unwrap();
        assert_eq!(locations.values[0].value, "Saudi Arabia");
        assert_eq!(locations.values[0].count, 2);
        let ports = groups.iter().find(|group| group.name == "PORTS").unwrap();
        assert_eq!(ports.values[0].value, "443");
        assert_eq!(ports.values[0].count, 2);
    }

    #[test]
    fn asset_type_filter_is_available() {
        let host = sample("host.example.com", "Example", &[443]);
        let mut certificate = sample("cert.example.com", "Example", &[443]);
        certificate.asset_type = "certificate".into();
        let assets = [host, certificate];
        let results = filter_assets(
            &assets,
            &AssetFilters {
                asset_type: Some("certificate"),
                ..AssetFilters::default()
            },
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].host, "cert.example.com");
    }
}
