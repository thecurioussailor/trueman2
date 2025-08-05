use actix_web::{get, post, HttpRequest, HttpResponse,HttpMessage, Responder};
use actix_web::web::Json;
use serde::Serialize;
use diesel::prelude::*;
use crate::jwt::Claims;
use uuid::Uuid;
use database::{
    establish_connection,
    NewBalance, Balance, Token,
    DepositRequest, WithdrawRequest,
    TransactionResponse,
    BalanceResponse,
    UserBalancesResponse,
    schema::{balances, tokens},
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

    let mut connection = establish_connection();

    let balances_result = balances::table
        .inner_join(tokens::table)
        .filter(balances::user_id.eq(user_id))
        .select((Balance::as_select(), Token::as_select()))
        .load::<(Balance, Token)>(&mut connection);

        match balances_result {
            Ok(balance_tokens) => {
                let balance_responses: Vec<BalanceResponse> = balance_tokens
                    .into_iter()
                    .map(|(balance, token)| BalanceResponse::from((balance, token)))
                    .collect();
    
                let user_balances = UserBalancesResponse {
                    user_id,
                    balances: balance_responses,
                };
    
                HttpResponse::Ok().json(SuccessResponse::new_single(
                    true,
                    "Balances retrieved successfully".to_string(),
                    Some(user_balances),
                ))
            }
            Err(e) => {
                println!("Error retrieving balances: {:?}", e);
                HttpResponse::InternalServerError().json(ErrorResponse::new(
                    "Error retrieving balances",
                ))
            }
        }

}

// POST /deposit - Deposit dummy funds
#[post("/deposit")]
pub async fn deposit_funds(req: HttpRequest, body: Json<DepositRequest>) -> impl Responder {
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

    // Validate deposit amount
    if body.amount <= 0 {
        let response = ErrorResponse::new("Deposit amount must be positive");
        return HttpResponse::BadRequest().json(response);
    }

    let mut connection = establish_connection();

    // Check if token exists and is active
    let token = match tokens::table
        .filter(tokens::id.eq(body.token_id))
        .filter(tokens::is_active.eq(true))
        .first::<Token>(&mut connection) {
        Ok(token) => token,
        Err(diesel::NotFound) => {
            let response = ErrorResponse::new("Token not found or inactive");
            return HttpResponse::BadRequest().json(response);
        }
        Err(e) => {
            println!("Error checking token: {:?}", e);
            let response = ErrorResponse::new("Error processing deposit");
            return HttpResponse::InternalServerError().json(response);
        }
    };

    // Try to update existing balance or create new one
    let balance_result = connection.transaction::<_, diesel::result::Error, _>(|conn| {
        // Check if balance already exists
        let existing_balance = balances::table
            .filter(balances::user_id.eq(user_id))
            .filter(balances::token_id.eq(body.token_id))
            .first::<Balance>(conn)
            .optional()?;

        match existing_balance {
            Some(balance) => {
                // Update existing balance
                let new_amount = balance.amount + body.amount;
                diesel::update(balances::table)
                    .filter(balances::id.eq(balance.id))
                    .set((
                        balances::amount.eq(new_amount),
                        balances::updated_at.eq(diesel::dsl::now),
                    ))
                    .execute(conn)?;
                Ok(new_amount)
            }
            None => {
                // Create new balance
                let new_balance = NewBalance::new(user_id, body.token_id, body.amount);
                diesel::insert_into(balances::table)
                    .values(&new_balance)
                    .execute(conn)?;
                Ok(body.amount)
            }
        }
    });

    match balance_result {
        Ok(new_balance) => {
            let transaction_response = TransactionResponse {
                success: true,
                message: format!("Successfully deposited {} {}", body.amount, token.symbol),
                new_balance: Some(new_balance),
            };

            HttpResponse::Ok().json(SuccessResponse::new_transaction(
                true,
                "Deposit successful".to_string(),
                Some(transaction_response),
            ))
        }
        Err(e) => {
            println!("Error processing deposit: {:?}", e);
            let response = ErrorResponse::new("Error processing deposit");
            HttpResponse::InternalServerError().json(response)
        }
    }
}

// POST /withdraw - Withdraw funds (limited to available balance)
#[post("/withdraw")]
pub async fn withdraw_funds(req: HttpRequest, body: Json<WithdrawRequest>) -> impl Responder {
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

    // Validate withdrawal amount
    if body.amount <= 0 {
        let response = ErrorResponse::new("Withdrawal amount must be positive");
        return HttpResponse::BadRequest().json(response);
    }

    let mut connection = establish_connection();

    // Check if token exists and is active
    let token = match tokens::table
        .filter(tokens::id.eq(body.token_id))
        .filter(tokens::is_active.eq(true))
        .first::<Token>(&mut connection) {
        Ok(token) => token,
        Err(diesel::NotFound) => {
            let response = ErrorResponse::new("Token not found or inactive");
            return HttpResponse::BadRequest().json(response);
        }
        Err(e) => {
            println!("Error checking token: {:?}", e);
            let response = ErrorResponse::new("Error processing withdrawal");
            return HttpResponse::InternalServerError().json(response);
        }
    };

    // Process withdrawal
    let withdrawal_result = connection.transaction::<_, diesel::result::Error, _>(|conn| {
        // Get current balance
        let balance = balances::table
            .filter(balances::user_id.eq(user_id))
            .filter(balances::token_id.eq(body.token_id))
            .first::<Balance>(conn)?;

        // Check if user has sufficient available balance
        if !balance.can_withdraw(body.amount) {
            return Err(diesel::result::Error::RollbackTransaction);
        }

        // Update balance
        let new_amount = balance.amount - body.amount;
        diesel::update(balances::table)
            .filter(balances::id.eq(balance.id))
            .set((
                balances::amount.eq(new_amount),
                balances::updated_at.eq(diesel::dsl::now),
            ))
            .execute(conn)?;

        Ok(new_amount)
    });

    match withdrawal_result {
        Ok(new_balance) => {
            let transaction_response = TransactionResponse {
                success: true,
                message: format!("Successfully withdrew {} {}", body.amount, token.symbol),
                new_balance: Some(new_balance),
            };

            HttpResponse::Ok().json(SuccessResponse::new_transaction(
                true,
                "Withdrawal successful".to_string(),
                Some(transaction_response),
            ))
        }
        Err(diesel::result::Error::RollbackTransaction) => {
            let response = ErrorResponse::new("Insufficient available balance");
            HttpResponse::BadRequest().json(response)
        }
        Err(diesel::NotFound) => {
            let response = ErrorResponse::new("No balance found for this token");
            HttpResponse::BadRequest().json(response)
        }
        Err(e) => {
            println!("Error processing withdrawal: {:?}", e);
            let response = ErrorResponse::new("Error processing withdrawal");
            HttpResponse::InternalServerError().json(response)
        }
    }
}