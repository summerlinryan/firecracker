// Copyright 2019 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::parsed_request::{Error, ParsedRequest, RequestAction};
use crate::request::Body;
use logger::{IncMetric, METRICS};
use micro_http::StatusCode;
use mmds::data_store::MmdsVersionType;
use vmm::rpc_interface::VmmAction::SetMmdsConfiguration;

pub(crate) fn parse_get_mmds(path_seconds_token: Option<&&str>) -> Result<ParsedRequest, Error> {
    match path_seconds_token {
        None => {
            METRICS.get_api_requests.mmds_count.inc();
            Ok(ParsedRequest::new(RequestAction::GetMMDS))
        }
        Some(&"version") => Ok(ParsedRequest::new(RequestAction::GetMMDSVersion)),
        Some(&unrecognized) => Err(Error::Generic(
            StatusCode::BadRequest,
            format!("Unrecognized GET request path `{}`.", unrecognized),
        )),
    }
}

pub(crate) fn parse_put_mmds(
    body: &Body,
    path_second_token: Option<&&str>,
) -> Result<ParsedRequest, Error> {
    METRICS.put_api_requests.mmds_count.inc();
    match path_second_token {
        None => Ok(ParsedRequest::new(RequestAction::PutMMDS(
            serde_json::from_slice(body.raw()).map_err(|e| {
                METRICS.put_api_requests.mmds_fails.inc();
                Error::SerdeJson(e)
            })?,
        ))),
        Some(&"config") => Ok(ParsedRequest::new_sync(SetMmdsConfiguration(
            serde_json::from_slice(body.raw()).map_err(|e| {
                METRICS.put_api_requests.mmds_fails.inc();
                Error::SerdeJson(e)
            })?,
        ))),
        Some(&"version") => {
            let version_type =
                serde_json::from_slice::<MmdsVersionType>(body.raw()).map_err(|e| {
                    METRICS.put_api_requests.mmds_fails.inc();
                    Error::SerdeJson(e)
                })?;
            Ok(ParsedRequest::new(RequestAction::SetMMDSVersion(
                version_type.version(),
            )))
        }
        Some(&unrecognized) => {
            METRICS.put_api_requests.mmds_fails.inc();
            Err(Error::Generic(
                StatusCode::BadRequest,
                format!("Unrecognized PUT request path `{}`.", unrecognized),
            ))
        }
    }
}

pub(crate) fn parse_patch_mmds(body: &Body) -> Result<ParsedRequest, Error> {
    METRICS.patch_api_requests.mmds_count.inc();
    Ok(ParsedRequest::new(RequestAction::PatchMMDS(
        serde_json::from_slice(body.raw()).map_err(|e| {
            METRICS.patch_api_requests.mmds_fails.inc();
            Error::SerdeJson(e)
        })?,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_get_mmds_request() {
        // Requests to `/mmds`.
        assert!(parse_get_mmds(None).is_ok());
        assert!(METRICS.get_api_requests.mmds_count.count() > 0);

        // Requests to `/mmds/version`.
        let path = "version";
        assert!(parse_get_mmds(Some(&path)).is_ok());

        // Requests to invalid path.
        assert!(parse_get_mmds(Some(&"invalid_path")).is_err());
    }

    #[test]
    fn test_parse_put_mmds_request() {
        let body = r#"{
                "foo": "bar"
              }"#;
        assert!(parse_put_mmds(&Body::new(body), None).is_ok());

        let invalid_body = "invalid_body";
        assert!(parse_put_mmds(&Body::new(invalid_body), None).is_err());
        assert!(METRICS.put_api_requests.mmds_fails.count() > 0);

        // Test `config` path.
        let body = r#"{
                "ipv4_address": "169.254.170.2"
              }"#;
        let config_path = "config";
        assert!(parse_put_mmds(&Body::new(body), Some(&config_path)).is_ok());

        let body = r#"{
                "ipv4_address": ""
              }"#;
        assert!(parse_put_mmds(&Body::new(body), Some(&config_path)).is_err());

        // Equivalent to reset the mmds configuration.
        let empty_body = r#"{}"#;
        assert!(parse_put_mmds(&Body::new(empty_body), Some(&config_path)).is_ok());

        // Test `version` path.
        let version_path = "version";
        let body = r#"{
                "version": "V1"
              }"#;
        assert!(parse_put_mmds(&Body::new(body), Some(&version_path)).is_ok());

        let body = r#"{
                "version": "V2"
              }"#;
        assert!(parse_put_mmds(&Body::new(body), Some(&version_path)).is_ok());
        let body = r#"{
                "version": "foo"
              }"#;
        assert!(parse_put_mmds(&Body::new(body), Some(&version_path)).is_err());

        let body = r#"{
                "version": ""
              }"#;
        assert!(parse_put_mmds(&Body::new(body), Some(&version_path)).is_err());

        let invalid_config_body = r#"{
                "invalid_config": "invalid_value"
              }"#;
        assert!(parse_put_mmds(&Body::new(invalid_config_body), Some(&config_path)).is_err());
        assert!(parse_put_mmds(&Body::new(invalid_config_body), Some(&version_path)).is_err());
        assert!(parse_put_mmds(&Body::new(body), Some(&"invalid_path")).is_err());
        assert!(parse_put_mmds(&Body::new(invalid_body), Some(&config_path)).is_err());
        assert!(parse_put_mmds(&Body::new(invalid_body), Some(&version_path)).is_err());
    }

    #[test]
    fn test_parse_patch_mmds_request() {
        let body = r#"{
                "foo": "bar"
              }"#;
        assert!(parse_patch_mmds(&Body::new(body)).is_ok());
        assert!(METRICS.patch_api_requests.mmds_count.count() > 0);
        assert!(parse_patch_mmds(&Body::new("invalid_body")).is_err());
        assert!(METRICS.patch_api_requests.mmds_fails.count() > 0);
    }
}
