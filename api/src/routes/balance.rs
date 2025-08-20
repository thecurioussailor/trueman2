use actix_web::{get, post, HttpRequest, HttpResponse,HttpMessage, Responder};
use actix_web::web::Json;
use serde::Serialize;
use diesel::prelude::*;
use crate::jwt::Claims;
use chrono::Utc;
use uuid::Uuid;
use crate::decimal_utils::{ to_atomic_units, ConversionError, convert_balances_to_decimal};
use crate::redis_manager::{
    get_redis_manager, EngineMessage, EngineProcessingResult, EngineResponse,
    BalanceRequest, BalanceOperation
};
use database::{
    DecimalDepositRequest, DecimalWithdrawRequest,
    TransactionResponse,
    UserBalancesResponse
};
#[derive(Serialize)]
struct ErrorResponse {
    status: bool,
    message: String,
    data: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct SuccessResponse {
    status: bool,
    message: String,
    data: Option<serde_json::Value>,
}

impl ErrorResponse {
    fn new(message: &str) -> Self {
        Self {
            status: false,
            message: message.to_string(),
            data: None,
        }
    }
}

impl SuccessResponse {
    fn new_single(status: bool, message: String, data: Option<UserBalancesResponse>) -> Self {
        Self {
            status,
            message,
            data: data.map(|market| serde_json::to_value(market).unwrap()),
        }
    }

    fn new_multiple(status: bool, message: String, data: Vec<UserBalancesResponse>) -> Self {
        Self {
            status,
            message,
            data: Some(serde_json::to_value(data).unwrap()),
        }
    }

    fn new_transaction(status: bool, message: String, data: Option<TransactionResponse>) -> Self {
        Self {
            status,
            message,
            data: data.map(|transaction| serde_json::to_value(transaction).unwrap()),
        }
    }
}

#[get("/balances")]
pub async fn get_user_balance(req: HttpRequest) -> impl Responder {

    let user_id = match req.extensions().get::<Claims>() {
        Some(claims) => {
            match Uuid::parse_str(&claims.user_id) {
                Ok(uuid) => uuid,
                Err(_) => {
                    let response = ErrorResponse::new("Invalid user ID format");
                    return HttpResponse::BadRequest().json(response);
                }
            }
        },
        None => {
            let response = ErrorResponse::new("Unauthorized");
            return HttpResponse::Unauthorized().json(response);
        }
    };
    println!("user_id: {:?}", user_id);
    // Create balance request for engine
    let balance_request = BalanceRequest {
        request_id: Uuid::new_v4().to_string(),
        user_id,
        token_id: Uuid::nil(), // Not needed for balance query
        operation: BalanceOperation::GetBalances,
        amount: 0,
        timestamp: Utc::now().timestamp_millis(),
    };
    
    let redis_manager = get_redis_manager().await;

    match redis_manager.send_and_wait(EngineMessage::Balance(balance_request), 3).await {
        EngineProcessingResult::Success(EngineResponse::Balance(response)) => {
            if response.success {
                if let Some(atomic_balances) = response.balances {
                    match convert_balances_to_decimal(atomic_balances) {
                        Ok(decimal_response) => {
                            HttpResponse::Ok().json(decimal_response)
                        }
                        Err(e) => {
                            HttpResponse::InternalServerError().json(ErrorResponse::new(&format!("Conversion error: {}", e)))
                        }
                    }
                } else {
                    // No balances found
                    HttpResponse::Ok().json(crate::decimal_utils::DecimalBalanceResponse {
                        balances: vec![],
                    })
                }
            } else {
                HttpResponse::InternalServerError().json(response.message)
            }
        }
        EngineProcessingResult::Timeout => {
            HttpResponse::RequestTimeout().json("Balance request timed out")
        }
        EngineProcessingResult::Error(e) => {
            HttpResponse::InternalServerError().json(format!("Error: {}", e))
        }
        _ => {
            HttpResponse::InternalServerError().json("Unexpected response type")
        }
    }
}

#[post("/deposit")]
pub async fn deposit_funds(req: HttpRequest, body: Json<DecimalDepositRequest>) -> impl Responder {
    let user_id = match req.extensions().get::<Claims>() {
        Some(claims) => {
            match Uuid::parse_str(&claims.user_id) {
                Ok(uuid) => uuid,
                Err(_) => {
                    let response = ErrorResponse::new("Invalid user ID format");
                    return HttpResponse::BadRequest().json(response);
                }
            }
        },
        None => {
            let response = ErrorResponse::new("Unauthorized");
            return HttpResponse::Unauthorized().json(response);
        }
    };
    let body = body.into_inner();
    
    // Validate request
    if body.amount <= 0.0 {
        return HttpResponse::BadRequest().json("Invalid amount");
    }

    let atomic_amount = match to_atomic_units(body.amount, body.token_id) {
        Ok(amount) => amount,
        Err(ConversionError::TokenNotFound) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new("Token not found"));
        },
        Err(ConversionError::InvalidAmount) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new("Invalid amount"));
        },
        Err(ConversionError::Overflow) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new("Amount too large"));
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse::new(&format!("Conversion error: {}", e)));
        }
    };
    
    // Create deposit request for engine
    let deposit_request = BalanceRequest {
        request_id: Uuid::new_v4().to_string(),
        user_id,
        token_id: body.token_id,
        operation: BalanceOperation::Deposit,
        amount: atomic_amount,
        timestamp: Utc::now().timestamp_millis(),
    };
    
    // Send to engine via Redis queue
    let redis_manager = get_redis_manager().await;
    match redis_manager.send_and_wait(EngineMessage::Balance(deposit_request), 5).await {
        EngineProcessingResult::Success(EngineResponse::Balance(response)) => {
            HttpResponse::Ok().json(TransactionResponse {
                success: response.success,
                message: response.message,
                new_balance: Some(response.new_balance),
            })
        }
        EngineProcessingResult::Timeout => {
            HttpResponse::Ok().json(TransactionResponse {
                success: true,
                message: "Deposit is being processed".to_string(),
                new_balance: None,
            })
        }
        EngineProcessingResult::Error(e) => {
            HttpResponse::InternalServerError().json(TransactionResponse {
                success: false,
                message: e,
                new_balance: None,
            })
        }
        _ => {
            HttpResponse::InternalServerError().json("Unexpected response type")
        }
    }
}

#[post("/withdraw")]
pub async fn withdraw_funds(req: HttpRequest, body: Json<DecimalWithdrawRequest>) -> impl Responder {
    let user_id = match req.extensions().get::<Claims>() {
        Some(claims) => {
            match Uuid::parse_str(&claims.user_id) {
                Ok(uuid) => uuid,
                Err(_) => {
                    let response = ErrorResponse::new("Invalid user ID format");
                    return HttpResponse::BadRequest().json(response);
                }
            }
        },
        None => {
            let response = ErrorResponse::new("Unauthorized");
            return HttpResponse::Unauthorized().json(response);
        }
    };

    let body = body.into_inner();

    if body.amount <= 0.0 {
        return HttpResponse::BadRequest().json("Withdrawal amount must be positive");
    }

     let atomic_amount = match to_atomic_units(body.amount, body.token_id) {
        Ok(amount) => amount,
        Err(ConversionError::TokenNotFound) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new("Token not found"));
        },
        Err(ConversionError::InvalidAmount) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new("Invalid amount"));
        },
        Err(ConversionError::Overflow) => {
            return HttpResponse::BadRequest().json(ErrorResponse::new("Amount too large"));
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse::new(&format!("Conversion error: {}", e)));
        }
    };

    // Create withdrawal request for engine
    let withdraw_request = BalanceRequest {
        request_id: Uuid::new_v4().to_string(),
        user_id,
        token_id: body.token_id,
        operation: BalanceOperation::Withdraw,
        amount: atomic_amount,
        timestamp: Utc::now().timestamp_millis(),
    };

    // Send unified message to engine
    let redis_manager = get_redis_manager().await;
    match redis_manager.send_and_wait(EngineMessage::Balance(withdraw_request), 5).await {
        EngineProcessingResult::Success(EngineResponse::Balance(response)) => {
            if response.success {
                HttpResponse::Ok().json(TransactionResponse {
                    success: true,
                    message: response.message,
                    new_balance: Some(response.new_balance),
                })
            } else {
                HttpResponse::BadRequest().json(TransactionResponse {
                    success: false,
                    message: response.message,
                    new_balance: None,
                })
            }
        }
        EngineProcessingResult::Timeout => {
            HttpResponse::Ok().json(TransactionResponse {
                success: true,
                message: "Withdrawal is being processed".to_string(),
                new_balance: None,
            })
        }
        EngineProcessingResult::Error(e) => {
            HttpResponse::BadRequest().json(TransactionResponse {
                success: false,
                message: e,
                new_balance: None,
            })
        }
        _ => {
            HttpResponse::InternalServerError().json("Unexpected response type")
        }
    }
}