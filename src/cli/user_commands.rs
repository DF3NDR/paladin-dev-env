/*
CLI Commands for User Management

Command-line interface for user operations, useful for administrative tasks
and testing.
*/

use crate::application::services::user_service::{
    UserService, UserRegistrationRequest, UserLoginRequest, UserProfileUpdateRequest
};
use crate::core::platform::container::user::{Email, UserProfile};
use clap::{Args, Subcommand};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Subcommand)]
pub enum UserCommands {
    /// Register a new user
    Register(RegisterArgs),
    /// Login user
    Login(LoginArgs),
    /// Get user information
    Get(GetUserArgs),
    /// Update user profile
    Update(UpdateUserArgs),
    /// List users
    List(ListUsersArgs),
    /// Activate user
    Activate(ActivateUserArgs),
    /// Deactivate user
    Deactivate(DeactivateUserArgs),
    /// Verify user
    Verify(VerifyUserArgs),
}

#[derive(Debug, Args)]
pub struct RegisterArgs {
    /// Username
    #[arg(short, long)]
    pub username: String,
    
    /// Email address
    #[arg(short, long)]
    pub email: String,
    
    /// Password
    #[arg(short, long)]
    pub password: String,
    
    /// First name
    #[arg(long)]
    pub first_name: Option<String>,
    
    /// Last name
    #[arg(long)]
    pub last_name: Option<String>,
    
    /// Bio
    #[arg(long)]
    pub bio: Option<String>,
    
    /// Timezone
    #[arg(long, default_value = "UTC")]
    pub timezone: String,
    
    /// Locale
    #[arg(long, default_value = "en-US")]
    pub locale: String,
}

#[derive(Debug, Args)]
pub struct LoginArgs {
    /// Email address
    #[arg(short, long)]
    pub email: String,
    
    /// Password
    #[arg(short, long)]
    pub password: String,
}

#[derive(Debug, Args)]
pub struct GetUserArgs {
    /// User ID or email
    #[arg(short, long)]
    pub identifier: String,
}

#[derive(Debug, Args)]
pub struct UpdateUserArgs {
    /// User ID
    #[arg(short, long)]
    pub user_id: String,
    
    /// New username
    #[arg(long)]
    pub username: Option<String>,
    
    /// New email
    #[arg(long)]
    pub email: Option<String>,
    
    /// First name
    #[arg(long)]
    pub first_name: Option<String>,
    
    /// Last name
    #[arg(long)]
    pub last_name: Option<String>,
    
    /// Bio
    #[arg(long)]
    pub bio: Option<String>,
    
    /// Avatar URL
    #[arg(long)]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Args)]
pub struct ListUsersArgs {
    /// Filter by active status
    #[arg(long)]
    pub active: Option<bool>,
    
    /// Filter by verification status
    #[arg(long)]
    pub verified: Option<bool>,
    
    /// Limit number of results
    #[arg(short, long, default_value = "10")]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct ActivateUserArgs {
    /// User ID
    #[arg(short, long)]
    pub user_id: String,
}

#[derive(Debug, Args)]
pub struct DeactivateUserArgs {
    /// User ID
    #[arg(short, long)]
    pub user_id: String,
}

#[derive(Debug, Args)]
pub struct VerifyUserArgs {
    /// User ID
    #[arg(short, long)]
    pub user_id: String,
}

/// User command handler
pub struct UserCommandHandler {
    user_service: Arc<UserService>,
}

impl UserCommandHandler {
    pub fn new(user_service: Arc<UserService>) -> Self {
        Self { user_service }
    }

    pub async fn handle_command(&self, command: UserCommands) -> Result<(), Box<dyn std::error::Error>> {
        match command {
            UserCommands::Register(args) => self.register_user(args).await,
            UserCommands::Login(args) => self.login_user(args).await,
            UserCommands::Get(args) => self.get_user(args).await,
            UserCommands::Update(args) => self.update_user(args).await,
            UserCommands::List(args) => self.list_users(args).await,
            UserCommands::Activate(args) => self.activate_user(args).await,
            UserCommands::Deactivate(args) => self.deactivate_user(args).await,
            UserCommands::Verify(args) => self.verify_user(args).await,
        }
    }

    async fn register_user(&self, args: RegisterArgs) -> Result<(), Box<dyn std::error::Error>> {
        let profile = Some(UserProfile {
            first_name: args.first_name,
            last_name: args.last_name,
            bio: args.bio,
            avatar_url: None,
            timezone: Some(args.timezone),
            locale: Some(args.locale),
        });

        let request = UserRegistrationRequest {
            username: args.username,
            email: args.email,
            password: args.password,
            profile,
        };

        let user = self.user_service.register_user(request).await?;
        
        println!("✅ User registered successfully!");
        println!("   ID: {}", user.uuid);
        println!("   Username: {}", user.username());
        println!("   Email: {}", user.email());
        println!("   Active: {}", user.is_active());
        println!("   Verified: {}", user.is_verified());

        Ok(())
    }

    async fn login_user(&self, args: LoginArgs) -> Result<(), Box<dyn std::error::Error>> {
        let request = UserLoginRequest {
            email: args.email,
            password: args.password,
        };

        let result = self.user_service.login_user(request).await?;
        
        if result.success {
            println!("✅ Login successful!");
            println!("   User ID: {}", result.user_id);
            println!("   Username: {}", result.username);
            println!("   Email: {}", result.email);
            println!("   Verified: {}", result.is_verified);
        } else {
            println!("❌ Login failed!");
        }

        Ok(())
    }

    async fn get_user(&self, args: GetUserArgs) -> Result<(), Box<dyn std::error::Error>> {
        let user = if let Ok(uuid) = Uuid::parse_str(&args.identifier) {
            self.user_service.get_user_by_id(uuid).await?
        } else {
            self.user_service.get_user_by_email(&args.identifier).await?
        };

        match user {
            Some(user) => {
                println!("👤 User Information:");
                println!("   ID: {}", user.uuid);
                println!("   Username: {}", user.username());
                println!("   Email: {}", user.email());
                println!("   Active: {}", user.is_active());
                println!("   Verified: {}", user.is_verified());
                println!("   Created: {}", user.created.format("%Y-%m-%d %H:%M:%S UTC"));
                println!("   Modified: {}", user.modified.format("%Y-%m-%d %H:%M:%S UTC"));
                
                let profile = user.profile();
                if profile.first_name.is_some() || profile.last_name.is_some() {
                    println!("   Name: {} {}", 
                        profile.first_name.as_deref().unwrap_or(""), 
                        profile.last_name.as_deref().unwrap_or("")
                    );
                }
                if let Some(bio) = &profile.bio {
                    println!("   Bio: {}", bio);
                }
                if let Some(timezone) = &profile.timezone {
                    println!("   Timezone: {}", timezone);
                }
                if let Some(locale) = &profile.locale {
                    println!("   Locale: {}", locale);
                }
            }
            None => println!("❌ User not found: {}", args.identifier),
        }

        Ok(())
    }

    async fn update_user(&self, args: UpdateUserArgs) -> Result<(), Box<dyn std::error::Error>> {
        let user_id = Uuid::parse_str(&args.user_id)?;
        
        let profile = if args.first_name.is_some() || args.last_name.is_some() || 
                         args.bio.is_some() || args.avatar_url.is_some() {
            // Get current user to preserve existing values
            let current_user = self.user_service.get_user_by_id(user_id).await?
                .ok_or("User not found")?;
            
            Some(UserProfile {
                first_name: args.first_name.or(current_user.profile().first_name.clone()),
                last_name: args.last_name.or(current_user.profile().last_name.clone()),
                bio: args.bio.or(current_user.profile().bio.clone()),
                avatar_url: args.avatar_url.or(current_user.profile().avatar_url.clone()),
                timezone: current_user.profile().timezone.clone(),
                locale: current_user.profile().locale.clone(),
            })
        } else {
            None
        };

        let request = UserProfileUpdateRequest {
            user_id,
            username: args.username,
            email: args.email,
            profile,
        };

        let user = self.user_service.update_user_profile(request).await?;
        
        println!("✅ User updated successfully!");
        println!("   ID: {}", user.uuid);
        println!("   Username: {}", user.username());
        println!("   Email: {}", user.email());

        Ok(())
    }

    async fn list_users(&self, args: ListUsersArgs) -> Result<(), Box<dyn std::error::Error>> {
        // This would require additional methods in UserService and UserRepository
        // For now, we'll implement a basic version
        
        println!("📋 User List:");
        
        if let Some(active) = args.active {
            let users = self.user_service.find_by_active_status(active).await?;
            self.print_user_list(&users, args.limit);
        } else if let Some(verified) = args.verified {
            let users = self.user_service.find_by_verification_status(verified).await?;
            self.print_user_list(&users, args.limit);
        } else {
            println!("   Use --active or --verified filters to list users");
        }

        Ok(())
    }

    fn print_user_list(&self, users: &[crate::core::platform::container::user::User], limit: u32) {
        let displayed_users = users.iter().take(limit as usize);
        
        for user in displayed_users {
            println!("   • {} ({}) - {} - Active: {} - Verified: {}", 
                user.username(), 
                user.email(), 
                user.uuid,
                user.is_active(),
                user.is_verified()
            );
        }
        
        if users.len() > limit as usize {
            println!("   ... and {} more users", users.len() - limit as usize);
        }
        
        println!("   Total: {} users", users.len());
    }

    async fn activate_user(&self, args: ActivateUserArgs) -> Result<(), Box<dyn std::error::Error>> {
        let user_id = Uuid::parse_str(&args.user_id)?;
        self.user_service.activate_user(user_id).await?;
        println!("✅ User activated successfully: {}", user_id);
        Ok(())
    }

    async fn deactivate_user(&self, args: DeactivateUserArgs) -> Result<(), Box<dyn std::error::Error>> {
        let user_id = Uuid::parse_str(&args.user_id)?;
        self.user_service.deactivate_user(user_id).await?;
        println!("✅ User deactivated successfully: {}", user_id);
        Ok(())
    }

    async fn verify_user(&self, args: VerifyUserArgs) -> Result<(), Box<dyn std::error::Error>> {
        let user_id = Uuid::parse_str(&args.user_id)?;
        self.user_service.verify_user(user_id).await?;
        println!("✅ User verified successfully: {}", user_id);
        Ok(())
    }
}

// Add methods to UserService to support CLI commands
impl UserService {
    pub async fn find_by_active_status(&self, is_active: bool) -> Result<Vec<crate::core::platform::container::user::User>, crate::core::platform::container::user::UserError> {
        self.user_repository.find_by_active_status(is_active).await
    }

    pub async fn find_by_verification_status(&self, is_verified: bool) -> Result<Vec<crate::core::platform::container::user::User>, crate::core::platform::container::user::UserError> {
        self.user_repository.find_by_verification_status(is_verified).await
    }

    pub async fn count_users(&self) -> Result<u64, crate::core::platform::container::user::UserError> {
        self.user_repository.count_users().await
    }
}