use actix_web::{post, web::{ Json}, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use diesel::prelude::*;
use database::{establish_connection, NewUser, schema::users, User};
use crate::jwt::create_jwt;

#[derive(Deserialize, Serialize, Debug)]
struct SignupRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct SignupResponse {
    message: String,
    email: String,
}


#[post("/signup")]
pub async fn signup(body: Json<SignupRequest>) -> impl Responder {
    let body = body.into_inner();

    let password_hash = bcrypt::hash(&body.password, 10).unwrap();

    let new_user = NewUser {
        email: body.email,
        password_hash,
    };

    let mut connection = establish_connection();

    match diesel::insert_into(users::table)
        .values(&new_user)
        .execute(&mut connection) {
            Ok(_) => {
                HttpResponse::Ok().json(SignupResponse {
                    message: "User created successfully".to_string(),
                    email: new_user.email,
                })
            }
            Err(e) => {
                println!("Error saving new user: {:?}", e);
                HttpResponse::InternalServerError().json("Error saving new user")
            }
        }
}

#[derive(Deserialize, Serialize, Debug)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    message: String,
    email: String,
    token: String,
}


#[post("/login")]
pub async fn login(body: Json<LoginRequest>) -> impl Responder {
    let body = body.into_inner();

    let mut connection = establish_connection();

    let user = match users::table
        .filter(users::email.eq(body.email))
        .select(User::as_select())
        .first::<User>(&mut connection) {
            Ok(user) => user,
            Err(e) => {
                println!("Error finding user: {:?}", e);
                return HttpResponse::InternalServerError().json("Error finding user");
            }
        };

    let password_matches = bcrypt::verify(&body.password, &user.password_hash).unwrap();

    if !password_matches {
        return HttpResponse::Unauthorized().json("Invalid password")
    } 

    let token = create_jwt(user.id.to_string(), user.email.clone(), user.is_admin).unwrap();

    HttpResponse::Ok().json(LoginResponse {
        message: "Login successful".to_string(),
        email: user.email,
        token,
    })
}

#[post("/admin/login")]
pub async fn admin_login(body: Json<LoginRequest>) -> impl Responder {
    let body = body.into_inner();

    let mut connection = establish_connection();

    let user = match users::table
        .filter(users::email.eq(body.email))
        .select(User::as_select())
        .first::<User>(&mut connection) {
            Ok(user) => user,
            Err(e) => {
                println!("Error finding user: {:?}", e);
                return HttpResponse::InternalServerError().json("Error finding user");
            }
        };  
    
    let password_matches = bcrypt::verify(&body.password, &user.password_hash).unwrap();

    if !password_matches {
        return HttpResponse::Unauthorized().json("Invalid password")
    } 

    if !user.is_admin {
        return HttpResponse::Unauthorized().json("User is not an admin");
    }

    let token = create_jwt(user.id.to_string(), user.email.clone(), user.is_admin).unwrap();

    HttpResponse::Ok().json(LoginResponse {
        message: "Admin login successful".to_string(),
        email: user.email,
        token,
    })
}