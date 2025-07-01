# REST API Usage Examples:

1. Register a new user:
POST /users/register
```json
{
    "username": "johndoe",
    "email": "john@example.com",
    "password": "secure_password123",
    "first_name": "John",
    "last_name": "Doe",
    "bio": "Software developer",
    "timezone": "America/New_York",
    "locale": "en-US"
}
```

2. Login:
POST /users/login
```
{
    "email": "john@example.com",
    "password": "secure_password123"
}
```

3. Get user:
GET /users/{user_id}

4. Update user profile:
PUT /users/{user_id}
```
{
    "username": "johnsmith",
    "first_name": "John",
    "last_name": "Smith",
    "bio": "Senior Software Developer"
}
```

5. Activate user:
POST /users/{user_id}/activate

6. Verify user:
POST /users/{user_id}/verify

CLI Usage Examples:

1. Register user:
./in4me user register -u johndoe -e john@example.com -p secure_password123 --first-name John --last-name Doe

2. Login:
./in4me user login -e john@example.com -p secure_password123

3. Get user:
./in4me user get -i john@example.com
./in4me user get -i 550e8400-e29b-41d4-a716-446655440000

4. Update user:
./in4me user update -u 550e8400-e29b-41d4-a716-446655440000 --username johnsmith --first-name John

5. List active users:
./in4me user list --active true --limit 20

6. Activate user:
./in4me user activate -u 550e8400-e29b-41d4-a716-446655440000

7. Verify user:
./in4me user verify -u 550e8400-e29b-41d4-a716-446655440000
*/

// =============================================================================
// INTEGRATION NOTES
// =============================================================================

/*
Integration Checklist:

1. ✅ Domain Layer - User entity built on Node with Email value object
2. ✅ Application Layer - UserService with business logic
3. ✅ Infrastructure Layer - SQLite repository implementation  
4. ✅ Presentation Layer - REST API endpoints
5. ✅ CLI Commands - Command-line interface
6. ✅ Integration - Service factory and dependency injection
7. ✅ Testing - Unit and integration tests
8. ✅ Error Handling - Comprehensive UserError types
9. ✅ Security - Argon2 password hashing
10. ✅ Logging - Integration with LogPort
11. ✅ Notifications - Welcome email via existing NotificationPublisherService

Files to create/update:
- src/core/platform/container/user.rs (new)
- src/application/services/user_service.rs (new)
- src/application/ports/output/user_repository_port.rs (new)
- src/infrastructure/repositories/sqlite_user_repository.rs (new)
- src/infrastructure/web/user_controller.rs (new)
- src/cli/user_commands.rs (new)
- src/config/user_config.rs (new)
- Update src/config/setup/service_runner.rs
- Update Cargo.toml with dependencies

Integration with Existing Services:
- ✅ Uses existing NotificationPublisherService from notification_port.rs
- ✅ Uses existing LogPort for logging
- ✅ Uses existing Settings struct for configuration
- ✅ Uses existing Node infrastructure for versioning
- ✅ Uses existing Message system for event publishing

Database Migration:
The SQLite repository automatically creates the users table with proper indexes.
The table schema includes all necessary fields and follows the Node pattern.

Security Features:
- Argon2 password hashing with salt
- Email validation with comprehensive regex
- Username validation rules
- Input sanitization and validation
- Proper error handling without information leakage

Versioning Support:
The User type is built on Node, automatically inheriting versioning capabilities.
All user changes can be tracked through the existing versioning system.

Integration Points:
- LogPort for user action logging (existing)
- NotificationPublisherService for welcome emails (existing)
- Settings struct for database configuration (existing)
- Existing Node infrastructure for versioning (existing)
- Message system for event publishing (existing)

This implementation provides a complete, production-ready user management system
that seamlessly integrates with your existing in4me framework architecture.
*/_123").is_ok());
        assert!(user_service.validate_username("test-user").is_ok());

        // Invalid usernames
        assert!(user_service.validate_username("").is_err());
        assert!(user_service.validate_username("ab").is_err());
        assert!(user_service.validate_username("user