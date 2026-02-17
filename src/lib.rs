use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

pub mod iam;
pub mod messaging;
pub mod provisioning;
pub mod shared;
pub mod tenancy;

#[derive(OpenApi)]
#[openapi(
    info(
        description = "auth self hosted platform"
    ),
    paths(
        iam::admin_identity::interfaces::rest::controllers::admin_authentication_controller::login_admin,
        iam::admin_identity::interfaces::rest::controllers::admin_authentication_controller::logout_admin,
        iam::identity::interfaces::rest::controllers::identity_controller::register_identity,
        iam::identity::interfaces::rest::controllers::identity_controller::confirm_registration,
        iam::identity::interfaces::rest::controllers::identity_controller::request_password_reset,
        iam::identity::interfaces::rest::controllers::identity_controller::reset_password,
        iam::authentication::interfaces::rest::controllers::authentication_controller::signin,
        iam::authentication::interfaces::rest::controllers::authentication_controller::logout,
        iam::authentication::interfaces::rest::controllers::authentication_controller::refresh_token,
        iam::authentication::interfaces::rest::controllers::authentication_controller::verify_token,
        iam::federation::interfaces::rest::controllers::google_controller::redirect_to_google,
        iam::federation::interfaces::rest::controllers::google_controller::google_callback,
        tenancy::interfaces::rest::controllers::tenant_controller::create_tenant,
        tenancy::interfaces::rest::controllers::tenant_controller::list_tenants,
        tenancy::interfaces::rest::controllers::tenant_controller::get_tenant,
        tenancy::interfaces::rest::controllers::tenant_controller::delete_tenant,
        tenancy::interfaces::rest::controllers::tenant_controller::rotate_google_oauth_config,
        tenancy::interfaces::rest::controllers::tenant_controller::rotate_tenant_jwt_signing_key,
        tenancy::interfaces::rest::controllers::tenant_controller::reissue_tenant_anon_key,
        tenancy::interfaces::rest::controllers::tenant_controller::update_tenant_frontend_url
    ),
    components(
        schemas(
            iam::admin_identity::interfaces::rest::resources::admin_login_resource::AdminLoginRequest,
            iam::admin_identity::interfaces::rest::resources::admin_login_resource::AdminLoginResponse,
            iam::admin_identity::interfaces::rest::resources::admin_logout_resource::AdminLogoutRequest,
            iam::identity::interfaces::rest::resources::register_identity_resource::RegisterIdentityRequest,
            iam::identity::interfaces::rest::resources::register_identity_resource::RegisterIdentityResponse,
            iam::identity::domain::model::commands::confirm_registration_command::ConfirmRegistrationCommand,
            iam::identity::interfaces::rest::resources::request_password_reset_resource::RequestPasswordResetRequest,
            iam::identity::interfaces::rest::resources::request_password_reset_resource::RequestPasswordResetResponse,
            iam::identity::interfaces::rest::resources::reset_password_resource::ResetPasswordRequest,
            iam::identity::interfaces::rest::resources::reset_password_resource::ResetPasswordResponse,
            iam::authentication::interfaces::rest::resources::signin_resource::SigninResource,
            iam::authentication::interfaces::rest::resources::signin_resource::TokenResponse,
            iam::authentication::interfaces::rest::resources::logout_resource::LogoutResource,
            iam::authentication::interfaces::rest::resources::refresh_token_resource::RefreshTokenResource,
            iam::authentication::interfaces::rest::resources::verify_token_resource::VerifyTokenResource,
            iam::authentication::interfaces::rest::resources::verify_token_resource::VerifyTokenResponse,
            iam::federation::interfaces::rest::resources::google_callback_query::GoogleCallbackQuery,
            tenancy::interfaces::rest::resources::create_tenant_resource::CreateTenantRequest,
            tenancy::interfaces::rest::resources::create_tenant_resource::CreateTenantResponse,
            tenancy::interfaces::rest::resources::tenant_resource::TenantResource,
            tenancy::interfaces::rest::resources::db_strategy_type_resource::DbStrategyTypeResource,
            tenancy::interfaces::rest::resources::rotate_google_oauth_config_resource::RotateGoogleOauthConfigRequest,
            tenancy::interfaces::rest::resources::rotate_google_oauth_config_resource::RotateGoogleOauthConfigResponse,
            tenancy::interfaces::rest::resources::rotate_tenant_jwt_signing_key_resource::RotateTenantJwtSigningKeyResponse,
            tenancy::interfaces::rest::resources::reissue_tenant_anon_key_resource::ReissueTenantAnonKeyResponse,
            tenancy::interfaces::rest::resources::update_tenant_frontend_url_resource::UpdateTenantFrontendUrlRequest,
            tenancy::interfaces::rest::resources::update_tenant_frontend_url_resource::UpdateTenantFrontendUrlResponse,
            shared::interfaces::rest::error_response::ErrorResponse
        )
    ),
    tags(
        (name = "admin-auth", description = "Admin authentication"),
        (name = "identity", description = "Identity management"),
        (name = "auth", description = "Authentication"),
        (name = "tenancy", description = "Tenant management")
    ),
    modifiers(&AdminSecurityAddon)
)]
pub struct ApiDoc;

struct AdminSecurityAddon;

impl Modify for AdminSecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "admin_bearer",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("Admin JWT token from POST /api/v1/admin/login"))
                        .build(),
                ),
            );
        }
    }
}
