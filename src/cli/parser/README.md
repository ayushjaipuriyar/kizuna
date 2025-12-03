# CLI Parser Module

This module provides comprehensive command-line argument parsing, validation, and routing for the Kizuna CLI.

## Components

### 1. ClapCommandParser (`clap_parser.rs`)
- **Purpose**: Parse command-line arguments using the clap framework
- **Features**:
  - Supports all Kizuna commands: discover, send, receive, stream, exec, peers, status, clipboard, tui, config
  - Comprehensive argument parsing with validation
  - Built-in help system with usage examples
  - Command suggestion for typos using Levenshtein distance
  - Subcommand support for complex commands (stream, clipboard, config)

### 2. CommandValidator (`validator.rs`)
- **Purpose**: Validate parsed commands with detailed error messages and suggestions
- **Features**:
  - Command-specific validation logic
  - File existence checking for send commands
  - Path validation for receive and stream commands
  - Security warnings (e.g., disabled encryption)
  - Dangerous command detection for exec
  - Contextual help generation
  - Similar command/option suggestions for typos

### 3. CommandRouter (`router.rs`)
- **Purpose**: Route validated commands to appropriate handlers
- **Features**:
  - Command execution context management
  - Routing to command-specific handlers
  - Validation warning display
  - Execution time tracking
  - Error handling and recovery strategies
  - Placeholder implementations for all commands

### 4. CommandExecutor (`integration.rs`)
- **Purpose**: Integrate all components into a complete execution flow
- **Features**:
  - End-to-end command execution from raw arguments to results
  - Automatic error recovery
  - Help text generation
  - Command suggestion for invalid commands

## Usage Example

```rust
use kizuna::cli::parser::CommandExecutor;

#[tokio::main]
async fn main() {
    let executor = CommandExecutor::new();
    
    // Execute a command
    let args = vec![
        "kizuna".to_string(),
        "discover".to_string(),
        "--type".to_string(),
        "desktop".to_string(),
    ];
    
    match executor.execute_from_args(args).await {
        Ok(result) => {
            println!("Success: {:?}", result.output);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
```

## Command Structure

All commands follow this structure:
```
kizuna <command> [subcommand] [arguments] [options] [flags]
```

### Available Commands

1. **discover** - Discover available peers
   - Options: `--type`, `--name`, `--timeout`, `--format`
   - Flags: `--watch`, `--json`

2. **send** - Send files to a peer
   - Arguments: file paths
   - Options: `--peer`
   - Flags: `--no-compression`, `--no-encryption`, `--verbose`

3. **receive** - Receive incoming file transfers
   - Options: `--output`, `--from`
   - Flags: `--auto-accept`

4. **stream** - Manage media streaming
   - Subcommands: `camera`
   - Options: `--camera`, `--quality`, `--output`
   - Flags: `--record`

5. **exec** - Execute command on remote peer
   - Arguments: command to execute
   - Options: `--peer`
   - Flags: `--interactive`

6. **peers** - List connected peers
   - Options: `--filter`, `--format`
   - Flags: `--watch`

7. **status** - Show system status
   - Flags: `--detailed`, `--json`

8. **clipboard** - Manage clipboard sharing
   - Subcommands: `share`, `status`, `history`
   - Options: `--peer`
   - Flags: `--enable`, `--disable`

9. **tui** - Launch interactive TUI

10. **config** - Manage configuration
    - Subcommands: `get`, `set`, `list`
    - Arguments: key, value

## Validation Features

The validator provides:
- **File validation**: Checks if files exist before sending
- **Path validation**: Validates output directories
- **Security warnings**: Warns about disabled encryption/compression
- **Dangerous command detection**: Warns about potentially destructive operations
- **Type validation**: Ensures valid device types, quality settings, etc.
- **Helpful suggestions**: Provides actionable suggestions for fixing issues

## Error Handling

The module provides comprehensive error handling:
- Parse errors with helpful messages
- Missing argument errors with clear indication
- Invalid argument value errors with reasons
- Command suggestions for typos
- Contextual help for invalid usage

## Testing

All components include unit tests:
- Command parsing tests
- Validation tests
- Routing tests
- Integration tests
- Levenshtein distance tests

Run tests with:
```bash
cargo test --lib cli::parser
```

## Future Enhancements

The current implementation provides placeholder handlers. Future work will:
1. Integrate with actual Kizuna core systems
2. Implement real command handlers
3. Add progress reporting for long-running operations
4. Enhance error recovery strategies
5. Add more sophisticated validation rules
