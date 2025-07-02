pub mod user_commands;

/*
CLI Module Tests

Tests for the CLI user commands module to ensure proper integration
with the User service and correct command line argument parsing.
*/

#[cfg(test)]
mod cli_tests {
    use crate::cli::user_commands::UserCommands;
    use clap::Parser;

    #[derive(Parser)]
    #[command(name = "test")]
    struct TestCli {
        #[command(subcommand)]
        user: Option<UserCommands>,
    }

    #[test]
    fn test_register_command_parsing() {
        let args = vec![
            "test", "register", 
            "--username", "testuser",
            "--email", "test@example.com",
            "--password", "securepassword",
            "--first-name", "Test",
            "--last-name", "User",
            "--bio", "Test bio"
        ];

        let cli = TestCli::try_parse_from(args).unwrap();
        
        if let Some(UserCommands::Register(register_args)) = cli.user {
            assert_eq!(register_args.username, "testuser");
            assert_eq!(register_args.email, "test@example.com");
            assert_eq!(register_args.password, "securepassword");
            assert_eq!(register_args.first_name, Some("Test".to_string()));
            assert_eq!(register_args.last_name, Some("User".to_string()));
            assert_eq!(register_args.bio, Some("Test bio".to_string()));
            assert_eq!(register_args.timezone, "UTC");
            assert_eq!(register_args.locale, "en-US");
        } else {
            panic!("Expected Register command");
        }
    }

    #[test]
    fn test_login_command_parsing() {
        let args = vec![
            "test", "login",
            "--email", "test@example.com",
            "--password", "securepassword"
        ];

        let cli = TestCli::try_parse_from(args).unwrap();
        
        if let Some(UserCommands::Login(login_args)) = cli.user {
            assert_eq!(login_args.email, "test@example.com");
            assert_eq!(login_args.password, "securepassword");
        } else {
            panic!("Expected Login command");
        }
    }

    #[test]
    fn test_get_user_command_parsing() {
        let args = vec![
            "test", "get",
            "--identifier", "user123"
        ];

        let cli = TestCli::try_parse_from(args).unwrap();
        
        if let Some(UserCommands::Get(get_args)) = cli.user {
            assert_eq!(get_args.identifier, "user123");
        } else {
            panic!("Expected Get command");
        }
    }

    #[test]
    fn test_register_command_with_minimal_args() {
        let args = vec![
            "test", "register",
            "--username", "testuser",
            "--email", "test@example.com", 
            "--password", "securepassword"
        ];

        let cli = TestCli::try_parse_from(args).unwrap();
        
        if let Some(UserCommands::Register(register_args)) = cli.user {
            assert_eq!(register_args.username, "testuser");
            assert_eq!(register_args.email, "test@example.com");
            assert_eq!(register_args.password, "securepassword");
            assert_eq!(register_args.first_name, None);
            assert_eq!(register_args.last_name, None);
            assert_eq!(register_args.bio, None);
            // Default values should be set
            assert_eq!(register_args.timezone, "UTC");
            assert_eq!(register_args.locale, "en-US");
        } else {
            panic!("Expected Register command");
        }
    }
}
