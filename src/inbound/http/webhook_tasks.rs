use anyhow::Result;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;
use url::Url;
use uuid::Uuid;

use crate::{
    domain::{
        hash_tasks::ports::HashTaskService,
        webhook_tasks::{
            model::{
                CreateWebhookTaskError, CreateWebhookTaskRequest, GetWebhookTaskRequests,
                WebhookTask, WebhookTaskBody, WebhookTaskDeadline, WebhookTaskId, WebhookTaskState,
                WebhookTaskUrl,
            },
            ports::WebhookTaskService,
        },
    },
    inbound::http::AppState,
};

pub async fn create_webhook_task<HS, WS>(
    State(state): State<AppState<HS, WS>>,
    Json(body): Json<CreateWebhookTaskHttpRequestBody>,
) -> Result<ApiSuccess<CreateWebhookTaskResponseData>, ApiError>
where
    HS: HashTaskService,
    WS: WebhookTaskService,
{
    info!("Received request to create webhook task");
    let req = body.try_into_domain()?;
    state
        .webhook_service
        .create_webhook_task(&req)
        .await
        .map_err(ApiError::from)
        .map(|ref webhook_task| ApiSuccess::new(StatusCode::CREATED, webhook_task.into()))
}

pub async fn get_webhook_tasks<HS, WS>(
    State(state): State<AppState<HS, WS>>,
    Query(query): Query<GetWebhookTaskHttpQueryParams>,
) -> Result<ApiSuccess<GetWebhookTaskResponseData>, ApiError>
where
    HS: HashTaskService,
    WS: WebhookTaskService,
{
    info!("Received request to get webhook task");
    let id = query.try_into_domain()?;
    state
        .webhook_service
        .get_webhook_tasks(&id)
        .await
        .map_err(ApiError::from)
        .map(|ref webhook_tasks| ApiSuccess::new(StatusCode::OK, webhook_tasks.into()))
}

// --- General purpose API request / response wrapper types

#[derive(Debug, Clone)]
pub struct ApiSuccess<T: Serialize + PartialEq>(StatusCode, Json<ApiResponseBody<T>>);

impl<T> PartialEq for ApiSuccess<T>
where
    T: Serialize + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1.0 == other.1.0
    }
}

impl<T: Serialize + PartialEq> ApiSuccess<T> {
    fn new(status: StatusCode, data: T) -> Self {
        ApiSuccess(status, Json(ApiResponseBody::new(status, data)))
    }
}

impl<T: Serialize + PartialEq> IntoResponse for ApiSuccess<T> {
    fn into_response(self) -> Response {
        (self.0, self.1).into_response()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    InternalServerError(String),
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        Self::InternalServerError(e.to_string())
    }
}

impl From<CreateWebhookTaskError> for ApiError {
    fn from(value: CreateWebhookTaskError) -> Self {
        match value {
            CreateWebhookTaskError::Unknown(cause) => {
                tracing::error!("{:?}\n{}", cause, cause.backtrace());
                Self::InternalServerError("Internal server error".to_string())
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::InternalServerError(e) => {
                tracing::error!("{}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponseBody::new_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal server error".to_string(),
                    )),
                )
                    .into_response()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiResponseBody<T: Serialize + PartialEq> {
    status_code: u16,
    data: T,
}

impl<T: Serialize + PartialEq> ApiResponseBody<T> {
    pub fn new(status_code: StatusCode, data: T) -> Self {
        Self {
            status_code: status_code.as_u16(),
            data,
        }
    }
}

impl ApiResponseBody<ApiErrorData> {
    pub fn new_error(status_code: StatusCode, message: String) -> Self {
        Self {
            status_code: status_code.as_u16(),
            data: ApiErrorData { message },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApiErrorData {
    pub message: String,
}

// --- Webhook-Task-Specific request / response types

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreateWebhookTaskHttpRequestBody {
    deadline: chrono::DateTime<Utc>,
    url: Url,
    body: String,
}

impl CreateWebhookTaskHttpRequestBody {
    /// Convert from HTTP type to Domain type.
    ///
    /// Better validation would go here
    fn try_into_domain(self) -> Result<CreateWebhookTaskRequest> {
        let deadline = WebhookTaskDeadline::from(self.deadline);
        let url = WebhookTaskUrl::from(self.url);
        let body = WebhookTaskBody::from(self.body);
        let request = CreateWebhookTaskRequest {
            deadline,
            url,
            body,
        };
        Ok(request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreateWebhookTaskResponseData {
    task_id: String,
}

impl From<&WebhookTask> for CreateWebhookTaskResponseData {
    fn from(value: &WebhookTask) -> Self {
        CreateWebhookTaskResponseData {
            task_id: value.id().to_string(),
        }
    }
}

//

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookTaskStateHttpQueryParams {
    Pending,
    Ready,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GetWebhookTaskHttpQueryParams {
    id: Option<Uuid>,
    state: Option<WebhookTaskStateHttpQueryParams>,
    limit: Option<u32>,
}

impl GetWebhookTaskHttpQueryParams {
    fn try_into_domain(self) -> Result<GetWebhookTaskRequests> {
        let id = self.id.map(|id| WebhookTaskId::from(id));
        let state = self.state.map(|state| match state {
            WebhookTaskStateHttpQueryParams::Pending => WebhookTaskState::Pending,
            WebhookTaskStateHttpQueryParams::Ready => WebhookTaskState::Ready,
            WebhookTaskStateHttpQueryParams::Finished => WebhookTaskState::Finished,
        });

        Ok(GetWebhookTaskRequests {
            id,
            state,
            limit: self.limit,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GetWebhookTaskResponseDataItem {
    deadline: chrono::DateTime<Utc>,
    url: Url,
    body: String,
}

impl From<&WebhookTask> for GetWebhookTaskResponseDataItem {
    fn from(value: &WebhookTask) -> Self {
        GetWebhookTaskResponseDataItem {
            deadline: value.deadline().utc().clone(),
            url: value.url().url().clone(),
            body: value.body().to_string().clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GetWebhookTaskResponseData {
    tasks: Vec<GetWebhookTaskResponseDataItem>,
}

impl From<&Vec<WebhookTask>> for GetWebhookTaskResponseData {
    fn from(value: &Vec<WebhookTask>) -> Self {
        GetWebhookTaskResponseData {
            tasks: value.iter().map(|task| task.into()).collect(),
        }
    }
}
