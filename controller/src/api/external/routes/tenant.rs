use route_recognizer;
use rusqlite::Connection;
use std::io;
use std::str::FromStr;
use std::sync::mpsc::Sender;
use tracing::{event, Level};

use crate::api;
use crate::api::external::services::element::elements_set_right_name;
use crate::api::types::element::OnlyId;
use crate::api::types::tenant::Tenant;
use crate::api::ApiChannel;
use crate::database::RikRepository;

pub fn get(
    _: &mut tiny_http::Request,
    _: &route_recognizer::Params,
    connection: &Connection,
    _: &Sender<ApiChannel>,
) -> Result<tiny_http::Response<io::Cursor<Vec<u8>>>, api::RikError> {
    if let Ok(mut tenants) = RikRepository::find_all(connection, "/tenant") {
        tenants = elements_set_right_name(tenants.clone());
        let tenants_json = serde_json::to_string(&tenants).unwrap();
        event!(Level::INFO, "tenants.get, tenants found");
        Ok(tiny_http::Response::from_string(tenants_json)
            .with_header(tiny_http::Header::from_str("Content-Type: application/json").unwrap())
            .with_status_code(tiny_http::StatusCode::from(200)))
    } else {
        Ok(tiny_http::Response::from_string("Cannot find tenant")
            .with_status_code(tiny_http::StatusCode::from(500)))
    }
}

pub fn create(
    req: &mut tiny_http::Request,
    _: &route_recognizer::Params,
    connection: &Connection,
    _: &Sender<ApiChannel>,
) -> Result<tiny_http::Response<io::Cursor<Vec<u8>>>, api::RikError> {
    let mut content = String::new();
    req.as_reader().read_to_string(&mut content).unwrap();
    let tenant: Tenant = serde_json::from_str(&content)?;

    if RikRepository::insert(connection, &tenant.name, &tenant.value).is_ok() {
        event!(Level::INFO, "Create tenant");
        Ok(tiny_http::Response::from_string(content)
            .with_header(tiny_http::Header::from_str("Content-Type: application/json").unwrap())
            .with_status_code(tiny_http::StatusCode::from(200)))
    } else {
        event!(Level::ERROR, "Cannot create tenant");
        Ok(tiny_http::Response::from_string("Cannot create tenant")
            .with_status_code(tiny_http::StatusCode::from(500)))
    }
}

pub fn delete(
    req: &mut tiny_http::Request,
    _: &route_recognizer::Params,
    connection: &Connection,
    _: &Sender<ApiChannel>,
) -> Result<tiny_http::Response<io::Cursor<Vec<u8>>>, api::RikError> {
    let mut content = String::new();
    req.as_reader().read_to_string(&mut content).unwrap();
    let OnlyId { id: delete_id } = serde_json::from_str(&content)?;

    if let Ok(tenant) = RikRepository::find_one(connection, &delete_id, "/tenant") {
        RikRepository::delete(connection, &tenant.id).unwrap();
        event!(Level::INFO, "Delete tenant");
        Ok(tiny_http::Response::from_string("").with_status_code(tiny_http::StatusCode::from(204)))
    } else {
        event!(Level::WARN, "Tenant id {} not found", delete_id);
        Ok(
            tiny_http::Response::from_string(format!("Tenant id {} not found", delete_id))
                .with_status_code(tiny_http::StatusCode::from(404)),
        )
    }
}
