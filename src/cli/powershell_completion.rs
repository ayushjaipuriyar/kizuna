// PowerShell-specific completion features and integration

use crate::cli::error::{CLIError, CLIResult};
use std::path::PathBuf;

/// PowerShell completion manager
pub struct PowerShellCompletion;

impl PowerShellCompletion {
    /// Generate enhanced PowerShell completion script with parameter hints
    pub fn generate_enhanced() -> CLIResult<String> {
        let script = r#"
# Kizuna PowerShell Completion Script
# Enhanced with parameter hints and Windows-specific features

using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'kizuna' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'kizuna'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
            }
            $element.Value
        }
    ) -join ';'

    $completions = @(switch ($command) {
        'kizuna' {
            [CompletionResult]::new('discover', 'discover', [CompletionResultType]::ParameterValue, 'Discover available peers on the network')
            [CompletionResult]::new('send', 'send', [CompletionResultType]::ParameterValue, 'Send files to a peer')
            [CompletionResult]::new('receive', 'receive', [CompletionResultType]::ParameterValue, 'Receive incoming file transfers')
            [CompletionResult]::new('stream', 'stream', [CompletionResultType]::ParameterValue, 'Manage media streaming')
            [CompletionResult]::new('exec', 'exec', [CompletionResultType]::ParameterValue, 'Execute command on remote peer')
            [CompletionResult]::new('peers', 'peers', [CompletionResultType]::ParameterValue, 'List connected peers')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show system status')
            [CompletionResult]::new('clipboard', 'clipboard', [CompletionResultType]::ParameterValue, 'Manage clipboard sharing')
            [CompletionResult]::new('tui', 'tui', [CompletionResultType]::ParameterValue, 'Launch interactive TUI')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Manage configuration')
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate shell completion scripts')
            break
        }
        'kizuna;discover' {
            [CompletionResult]::new('--type', '--type', [CompletionResultType]::ParameterName, 'Filter by device type (desktop, mobile, tablet)')
            [CompletionResult]::new('-t', '-t', [CompletionResultType]::ParameterName, 'Filter by device type')
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'Filter by device name (supports wildcards)')
            [CompletionResult]::new('-n', '-n', [CompletionResultType]::ParameterName, 'Filter by device name')
            [CompletionResult]::new('--timeout', '--timeout', [CompletionResultType]::ParameterName, 'Discovery timeout in seconds')
            [CompletionResult]::new('--watch', '--watch', [CompletionResultType]::ParameterName, 'Continuously watch for peers')
            [CompletionResult]::new('-w', '-w', [CompletionResultType]::ParameterName, 'Continuously watch for peers')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (table, json, csv, minimal)')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format')
            [CompletionResult]::new('-j', '-j', [CompletionResultType]::ParameterName, 'Output in JSON format')
            break
        }
        'kizuna;send' {
            [CompletionResult]::new('--peer', '--peer', [CompletionResultType]::ParameterName, 'Target peer name or ID')
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'Target peer name or ID')
            [CompletionResult]::new('--no-compression', '--no-compression', [CompletionResultType]::ParameterName, 'Disable compression')
            [CompletionResult]::new('--no-encryption', '--no-encryption', [CompletionResultType]::ParameterName, 'Disable encryption (not recommended)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Show detailed progress')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Show detailed progress')
            break
        }
        'kizuna;receive' {
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output directory for received files')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Output directory')
            [CompletionResult]::new('--auto-accept', '--auto-accept', [CompletionResultType]::ParameterName, 'Automatically accept transfers from trusted peers')
            [CompletionResult]::new('-a', '-a', [CompletionResultType]::ParameterName, 'Auto-accept from trusted peers')
            [CompletionResult]::new('--from', '--from', [CompletionResultType]::ParameterName, 'Only accept from specific peer')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Only accept from specific peer')
            break
        }
        'kizuna;stream' {
            [CompletionResult]::new('camera', 'camera', [CompletionResultType]::ParameterValue, 'Stream camera feed')
            break
        }
        'kizuna;stream;camera' {
            [CompletionResult]::new('--camera', '--camera', [CompletionResultType]::ParameterName, 'Camera device ID or index')
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Camera device ID')
            [CompletionResult]::new('--quality', '--quality', [CompletionResultType]::ParameterName, 'Stream quality (low, medium, high, ultra)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Stream quality')
            [CompletionResult]::new('--record', '--record', [CompletionResultType]::ParameterName, 'Record stream to file')
            [CompletionResult]::new('-r', '-r', [CompletionResultType]::ParameterName, 'Record stream')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Recording output file')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Recording output file')
            break
        }
        'kizuna;exec' {
            [CompletionResult]::new('--peer', '--peer', [CompletionResultType]::ParameterName, 'Target peer name or ID')
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'Target peer')
            [CompletionResult]::new('--interactive', '--interactive', [CompletionResultType]::ParameterName, 'Interactive mode with stdin support')
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Interactive mode')
            break
        }
        'kizuna;peers' {
            [CompletionResult]::new('--watch', '--watch', [CompletionResultType]::ParameterName, 'Continuously monitor peer status')
            [CompletionResult]::new('-w', '-w', [CompletionResultType]::ParameterName, 'Monitor peer status')
            [CompletionResult]::new('--filter', '--filter', [CompletionResultType]::ParameterName, 'Filter peers by criteria')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Filter peers')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (table, json, csv)')
            break
        }
        'kizuna;status' {
            [CompletionResult]::new('--detailed', '--detailed', [CompletionResultType]::ParameterName, 'Show detailed status information')
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'Detailed status')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format')
            [CompletionResult]::new('-j', '-j', [CompletionResultType]::ParameterName, 'JSON output')
            break
        }
        'kizuna;clipboard' {
            [CompletionResult]::new('share', 'share', [CompletionResultType]::ParameterValue, 'Toggle clipboard sharing')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show clipboard sharing status')
            [CompletionResult]::new('history', 'history', [CompletionResultType]::ParameterValue, 'View clipboard history')
            break
        }
        'kizuna;clipboard;share' {
            [CompletionResult]::new('--peer', '--peer', [CompletionResultType]::ParameterName, 'Specific peer to share with')
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'Specific peer')
            [CompletionResult]::new('--enable', '--enable', [CompletionResultType]::ParameterName, 'Enable clipboard sharing')
            [CompletionResult]::new('-e', '-e', [CompletionResultType]::ParameterName, 'Enable sharing')
            [CompletionResult]::new('--disable', '--disable', [CompletionResultType]::ParameterName, 'Disable clipboard sharing')
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'Disable sharing')
            break
        }
        'kizuna;config' {
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Get configuration value')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Set configuration value')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all configuration')
            break
        }
        'kizuna;completion' {
            [CompletionResult]::new('bash', 'bash', [CompletionResultType]::ParameterValue, 'Generate Bash completion')
            [CompletionResult]::new('zsh', 'zsh', [CompletionResultType]::ParameterValue, 'Generate Zsh completion')
            [CompletionResult]::new('fish', 'fish', [CompletionResultType]::ParameterValue, 'Generate Fish completion')
            [CompletionResult]::new('powershell', 'powershell', [CompletionResultType]::ParameterValue, 'Generate PowerShell completion')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
"#;

        Ok(script.to_string())
    }

    /// Generate PowerShell profile integration script
    pub fn generate_profile_integration() -> CLIResult<String> {
        let script = r#"
# Kizuna PowerShell Profile Integration
# Add this to your PowerShell profile ($PROFILE)

# Load Kizuna completion
if (Get-Command kizuna -ErrorAction SilentlyContinue) {
    kizuna completion powershell | Out-String | Invoke-Expression
}

# Optional: Add Kizuna aliases
Set-Alias -Name kz -Value kizuna

# Optional: Add helper functions
function Kizuna-Discover {
    param([string]$Type, [string]$Name)
    $args = @('discover')
    if ($Type) { $args += @('--type', $Type) }
    if ($Name) { $args += @('--name', $Name) }
    & kizuna @args
}

function Kizuna-Send {
    param(
        [Parameter(Mandatory=$true)]
        [string[]]$Files,
        [string]$Peer
    )
    $args = @('send') + $Files
    if ($Peer) { $args += @('--peer', $Peer) }
    & kizuna @args
}

function Kizuna-Receive {
    param([string]$Output, [switch]$AutoAccept)
    $args = @('receive')
    if ($Output) { $args += @('--output', $Output) }
    if ($AutoAccept) { $args += '--auto-accept' }
    & kizuna @args
}

# Export functions
Export-ModuleMember -Function Kizuna-Discover, Kizuna-Send, Kizuna-Receive
"#;

        Ok(script.to_string())
    }

    /// Get PowerShell installation instructions
    pub fn get_installation_instructions() -> String {
        r#"PowerShell Completion Installation Instructions
==============================================

Method 1: Automatic Installation (Recommended)
-----------------------------------------------
Run the following command in PowerShell:

    kizuna completion powershell | Out-String | Invoke-Expression

To make it permanent, add to your PowerShell profile:

    if (!(Test-Path $PROFILE)) { New-Item -Path $PROFILE -ItemType File -Force }
    Add-Content $PROFILE "`nkizuna completion powershell | Out-String | Invoke-Expression"

Method 2: Manual Installation
------------------------------
1. Generate the completion script:
   
   kizuna completion powershell > "$env:USERPROFILE\Documents\PowerShell\Scripts\kizuna-completion.ps1"

2. Add to your profile:
   
   if (!(Test-Path $PROFILE)) { New-Item -Path $PROFILE -ItemType File -Force }
   Add-Content $PROFILE "`n. `"$env:USERPROFILE\Documents\PowerShell\Scripts\kizuna-completion.ps1`""

3. Reload your profile:
   
   . $PROFILE

Method 3: Enhanced Installation with Helper Functions
------------------------------------------------------
1. Generate the enhanced profile integration:
   
   kizuna completion powershell --enhanced > "$env:USERPROFILE\Documents\PowerShell\Modules\Kizuna\Kizuna.psm1"

2. Import the module in your profile:
   
   if (!(Test-Path $PROFILE)) { New-Item -Path $PROFILE -ItemType File -Force }
   Add-Content $PROFILE "`nImport-Module Kizuna"

Verification
------------
After installation, restart PowerShell and type:

    kizuna <TAB>

You should see command completions with descriptions.

Troubleshooting
---------------
If completions don't work:

1. Check execution policy:
   Get-ExecutionPolicy
   
   If it's "Restricted", set it to "RemoteSigned":
   Set-ExecutionPolicy RemoteSigned -Scope CurrentUser

2. Verify kizuna is in PATH:
   Get-Command kizuna

3. Check if profile is loaded:
   Test-Path $PROFILE
   Get-Content $PROFILE

For more help, visit: https://github.com/kizuna/kizuna
"#.to_string()
    }

    /// Get the PowerShell profile path
    pub fn get_profile_path() -> CLIResult<PathBuf> {
        #[cfg(windows)]
        {
            let profile_dir = dirs::document_dir()
                .ok_or_else(|| CLIError::other("Could not determine documents directory"))?
                .join("PowerShell");

            Ok(profile_dir.join("Microsoft.PowerShell_profile.ps1"))
        }

        #[cfg(not(windows))]
        {
            let config_dir = dirs::config_dir()
                .ok_or_else(|| CLIError::other("Could not determine config directory"))?;

            Ok(config_dir.join("powershell").join("Microsoft.PowerShell_profile.ps1"))
        }
    }

    /// Check if PowerShell completion is installed
    pub fn is_installed() -> bool {
        if let Ok(profile_path) = Self::get_profile_path() {
            if let Ok(content) = std::fs::read_to_string(profile_path) {
                return content.contains("kizuna completion") || content.contains("kizuna-completion");
            }
        }
        false
    }

    /// Install PowerShell completion to profile
    pub fn install_to_profile() -> CLIResult<()> {
        let profile_path = Self::get_profile_path()?;

        // Create profile directory if it doesn't exist
        if let Some(parent) = profile_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CLIError::other(format!("Failed to create profile directory: {}", e))
            })?;
        }

        // Check if already installed
        if Self::is_installed() {
            return Err(CLIError::other(
                "Kizuna completion is already installed in PowerShell profile",
            ));
        }

        // Append to profile
        let integration = "\n# Kizuna completion\nkizuna completion powershell | Out-String | Invoke-Expression\n";

        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&profile_path)
            .and_then(|mut file| {
                use std::io::Write;
                file.write_all(integration.as_bytes())
            })
            .map_err(|e| CLIError::other(format!("Failed to write to profile: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_enhanced() {
        let result = PowerShellCompletion::generate_enhanced();
        assert!(result.is_ok());
        let script = result.unwrap();
        assert!(script.contains("Register-ArgumentCompleter"));
        assert!(script.contains("kizuna"));
        assert!(script.contains("discover"));
    }

    #[test]
    fn test_generate_profile_integration() {
        let result = PowerShellCompletion::generate_profile_integration();
        assert!(result.is_ok());
        let script = result.unwrap();
        assert!(script.contains("Kizuna-Discover"));
        assert!(script.contains("Kizuna-Send"));
        assert!(script.contains("Kizuna-Receive"));
    }

    #[test]
    fn test_get_installation_instructions() {
        let instructions = PowerShellCompletion::get_installation_instructions();
        assert!(instructions.contains("PowerShell"));
        assert!(instructions.contains("completion"));
        assert!(instructions.contains("$PROFILE"));
    }

    #[test]
    fn test_get_profile_path() {
        let result = PowerShellCompletion::get_profile_path();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("PowerShell"));
    }
}
