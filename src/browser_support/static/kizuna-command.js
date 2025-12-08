/**
 * Kizuna Command Execution API
 * 
 * Command execution functionality for browser clients.
 * Supports real-time output streaming, command history, and authorization.
 */

(function (global) {
    'use strict';

    /**
     * Command Execution Manager
     */
    class CommandExecutionManager {
        constructor(sdk) {
            this.sdk = sdk;
            this.activeCommands = new Map();
            this.commandHistory = [];
            this.savedTemplates = new Map();
            this.maxHistorySize = 100;

            // Listen for command execution messages
            this.sdk.on('message:CommandExecution', (msg) => this._handleCommandMessage(msg));
            this.sdk.on('message:CommandOutput', (msg) => this._handleOutputMessage(msg));
            this.sdk.on('message:CommandComplete', (msg) => this._handleCompleteMessage(msg));
            this.sdk.on('message:CommandError', (msg) => this._handleErrorMessage(msg));

            this.sdk._log('Command Execution Manager initialized');
        }

        /**
         * Execute a command on a peer
         * @param {string} command - Command to execute
         * @param {string} peerId - Target peer ID
         * @param {Object} options - Execution options
         * @returns {Promise<CommandExecution>} Command execution object
         */
        async executeCommand(command, peerId, options = {}) {
            const commandId = this.sdk._generateUUID();

            const execution = {
                id: commandId,
                command: command,
                peerId: peerId,
                status: 'pending',
                output: [],
                exitCode: null,
                startTime: Date.now(),
                endTime: null,
                error: null,
                workingDirectory: options.workingDirectory || null,
                environment: options.environment || {},
                timeout: options.timeout || null,
                onOutput: options.onOutput || null,
                onComplete: options.onComplete || null,
                onError: options.onError || null
            };

            this.activeCommands.set(commandId, execution);
            this.sdk._emit('commandStarted', { execution });

            try {
                // Send command execution request
                await this.sdk.sendMessage({
                    message_type: 'CommandExecution',
                    payload: {
                        command_id: commandId,
                        action: 'execute',
                        command: command,
                        peer_id: peerId,
                        working_directory: execution.workingDirectory,
                        environment: execution.environment,
                        timeout: execution.timeout
                    }
                }, 'command');

                execution.status = 'running';

                // Add to history
                this._addToHistory(command);

                return execution;
            } catch (error) {
                execution.status = 'failed';
                execution.error = error.message;
                this.sdk._emit('commandError', { execution, error });
                if (execution.onError) execution.onError(error);
                throw error;
            }
        }

        /**
         * Handle command messages
         * @private
         */
        _handleCommandMessage(message) {
            const payload = message.payload;
            const execution = this.activeCommands.get(payload.command_id);

            if (!execution) {
                this.sdk._log('Command execution not found:', payload.command_id);
                return;
            }

            switch (payload.action) {
                case 'accepted':
                    execution.status = 'running';
                    this.sdk._emit('commandAccepted', { execution });
                    break;

                case 'rejected':
                    execution.status = 'failed';
                    execution.error = payload.reason || 'Command rejected';
                    this.sdk._emit('commandError', {
                        execution,
                        error: new Error(execution.error)
                    });
                    if (execution.onError) {
                        execution.onError(new Error(execution.error));
                    }
                    break;

                case 'authorization_required':
                    execution.status = 'awaiting_authorization';
                    this.sdk._emit('commandAuthorizationRequired', {
                        execution,
                        authorizationInfo: payload.authorization_info
                    });
                    break;
            }
        }

        /**
         * Handle output messages
         * @private
         */
        _handleOutputMessage(message) {
            const payload = message.payload;
            const execution = this.activeCommands.get(payload.command_id);

            if (!execution) {
                return;
            }

            const outputLine = {
                type: payload.output_type || 'stdout',
                content: payload.content,
                timestamp: payload.timestamp || Date.now()
            };

            execution.output.push(outputLine);

            this.sdk._emit('commandOutput', { execution, output: outputLine });

            if (execution.onOutput) {
                execution.onOutput(outputLine);
            }
        }

        /**
         * Handle complete messages
         * @private
         */
        _handleCompleteMessage(message) {
            const payload = message.payload;
            const execution = this.activeCommands.get(payload.command_id);

            if (!execution) {
                return;
            }

            execution.status = 'completed';
            execution.exitCode = payload.exit_code;
            execution.endTime = Date.now();

            this.activeCommands.delete(execution.id);

            this.sdk._emit('commandComplete', { execution });

            if (execution.onComplete) {
                execution.onComplete(execution);
            }
        }

        /**
         * Handle error messages
         * @private
         */
        _handleErrorMessage(message) {
            const payload = message.payload;
            const execution = this.activeCommands.get(payload.command_id);

            if (!execution) {
                return;
            }

            execution.status = 'failed';
            execution.error = payload.error;
            execution.endTime = Date.now();

            this.activeCommands.delete(execution.id);

            this.sdk._emit('commandError', {
                execution,
                error: new Error(payload.error)
            });

            if (execution.onError) {
                execution.onError(new Error(payload.error));
            }
        }

        /**
         * Terminate a running command
         * @param {string} commandId - Command ID
         * @returns {Promise<void>}
         */
        async terminateCommand(commandId) {
            const execution = this.activeCommands.get(commandId);

            if (!execution) {
                throw new Error('Command not found');
            }

            if (execution.status !== 'running') {
                throw new Error('Command is not running');
            }

            await this.sdk.sendMessage({
                message_type: 'CommandExecution',
                payload: {
                    command_id: commandId,
                    action: 'terminate'
                }
            }, 'command');

            execution.status = 'terminated';
            execution.endTime = Date.now();

            this.activeCommands.delete(commandId);

            this.sdk._emit('commandTerminated', { execution });
        }

        /**
         * Send input to a running command
         * @param {string} commandId - Command ID
         * @param {string} input - Input to send
         * @returns {Promise<void>}
         */
        async sendInput(commandId, input) {
            const execution = this.activeCommands.get(commandId);

            if (!execution) {
                throw new Error('Command not found');
            }

            if (execution.status !== 'running') {
                throw new Error('Command is not running');
            }

            await this.sdk.sendMessage({
                message_type: 'CommandExecution',
                payload: {
                    command_id: commandId,
                    action: 'input',
                    input: input
                }
            }, 'command');

            this.sdk._log('Sent input to command:', commandId);
        }

        /**
         * Authorize a command execution
         * @param {string} commandId - Command ID
         * @param {boolean} approved - Whether to approve
         * @returns {Promise<void>}
         */
        async authorizeCommand(commandId, approved) {
            const execution = this.activeCommands.get(commandId);

            if (!execution) {
                throw new Error('Command not found');
            }

            await this.sdk.sendMessage({
                message_type: 'CommandExecution',
                payload: {
                    command_id: commandId,
                    action: 'authorize',
                    approved: approved
                }
            }, 'command');

            if (approved) {
                execution.status = 'running';
                this.sdk._emit('commandAuthorized', { execution });
            } else {
                execution.status = 'failed';
                execution.error = 'Authorization denied';
                this.activeCommands.delete(commandId);
                this.sdk._emit('commandAuthorizationDenied', { execution });
            }
        }

        /**
         * Get active command executions
         * @returns {Array} Active executions
         */
        getActiveCommands() {
            return Array.from(this.activeCommands.values());
        }

        /**
         * Get command by ID
         * @param {string} commandId - Command ID
         * @returns {Object|null} Command execution object
         */
        getCommand(commandId) {
            return this.activeCommands.get(commandId) || null;
        }

        /**
         * Add command to history
         * @private
         */
        _addToHistory(command) {
            // Don't add duplicates of the last command
            if (this.commandHistory.length > 0 &&
                this.commandHistory[this.commandHistory.length - 1] === command) {
                return;
            }

            this.commandHistory.push(command);

            // Trim history if too large
            if (this.commandHistory.length > this.maxHistorySize) {
                this.commandHistory.shift();
            }

            this.sdk._emit('commandHistoryUpdated', {
                history: this.commandHistory
            });
        }

        /**
         * Get command history
         * @returns {Array} Command history
         */
        getHistory() {
            return [...this.commandHistory];
        }

        /**
         * Clear command history
         */
        clearHistory() {
            this.commandHistory = [];
            this.sdk._emit('commandHistoryCleared');
        }

        /**
         * Search command history
         * @param {string} query - Search query
         * @returns {Array} Matching commands
         */
        searchHistory(query) {
            const lowerQuery = query.toLowerCase();
            return this.commandHistory.filter(cmd =>
                cmd.toLowerCase().includes(lowerQuery)
            );
        }

        /**
         * Save a command template
         * @param {string} name - Template name
         * @param {string} command - Command template
         * @param {Object} metadata - Template metadata
         */
        saveTemplate(name, command, metadata = {}) {
            this.savedTemplates.set(name, {
                name: name,
                command: command,
                metadata: metadata,
                createdAt: Date.now()
            });

            this.sdk._emit('templateSaved', { name, command });
            this.sdk._log('Template saved:', name);
        }

        /**
         * Get a saved template
         * @param {string} name - Template name
         * @returns {Object|null} Template object
         */
        getTemplate(name) {
            return this.savedTemplates.get(name) || null;
        }

        /**
         * Get all saved templates
         * @returns {Array} All templates
         */
        getAllTemplates() {
            return Array.from(this.savedTemplates.values());
        }

        /**
         * Delete a template
         * @param {string} name - Template name
         */
        deleteTemplate(name) {
            if (this.savedTemplates.delete(name)) {
                this.sdk._emit('templateDeleted', { name });
                this.sdk._log('Template deleted:', name);
            }
        }

        /**
         * Execute a saved template
         * @param {string} name - Template name
         * @param {string} peerId - Target peer ID
         * @param {Object} variables - Template variables
         * @param {Object} options - Execution options
         * @returns {Promise<CommandExecution>} Command execution object
         */
        async executeTemplate(name, peerId, variables = {}, options = {}) {
            const template = this.getTemplate(name);

            if (!template) {
                throw new Error(`Template '${name}' not found`);
            }

            // Replace variables in template
            let command = template.command;
            for (const [key, value] of Object.entries(variables)) {
                command = command.replace(new RegExp(`\\{${key}\\}`, 'g'), value);
            }

            return await this.executeCommand(command, peerId, options);
        }

        /**
         * Export templates
         * @returns {string} JSON string of templates
         */
        exportTemplates() {
            const templates = Array.from(this.savedTemplates.values());
            return JSON.stringify(templates, null, 2);
        }

        /**
         * Import templates
         * @param {string} jsonString - JSON string of templates
         */
        importTemplates(jsonString) {
            try {
                const templates = JSON.parse(jsonString);

                for (const template of templates) {
                    this.savedTemplates.set(template.name, template);
                }

                this.sdk._emit('templatesImported', { count: templates.length });
                this.sdk._log('Imported', templates.length, 'templates');
            } catch (error) {
                this.sdk._error('Failed to import templates:', error);
                throw error;
            }
        }

        /**
         * Set maximum history size
         * @param {number} size - Maximum size
         */
        setMaxHistorySize(size) {
            this.maxHistorySize = size;

            // Trim if necessary
            while (this.commandHistory.length > this.maxHistorySize) {
                this.commandHistory.shift();
            }
        }

        /**
         * Get command statistics
         * @returns {Object} Statistics
         */
        getStatistics() {
            return {
                activeCommands: this.activeCommands.size,
                historySize: this.commandHistory.length,
                savedTemplates: this.savedTemplates.size
            };
        }

        /**
         * Cleanup
         */
        destroy() {
            this.activeCommands.clear();
            this.sdk._log('Command Execution Manager destroyed');
        }
    }

    /**
     * Terminal Interface
     * 
     * Web-based terminal interface for command execution
     */
    class TerminalInterface {
        constructor(commandManager, container) {
            this.commandManager = commandManager;
            this.container = container;
            this.historyIndex = -1;
            this.currentExecution = null;

            this._setupUI();
            this._setupEventListeners();
        }

        /**
         * Setup terminal UI
         * @private
         */
        _setupUI() {
            this.container.innerHTML = `
                <div class="kizuna-terminal">
                    <div class="terminal-output" id="terminal-output"></div>
                    <div class="terminal-input-line">
                        <span class="terminal-prompt">$</span>
                        <input type="text" class="terminal-input" id="terminal-input" 
                               placeholder="Enter command..." autocomplete="off">
                    </div>
                </div>
            `;

            this.outputElement = this.container.querySelector('#terminal-output');
            this.inputElement = this.container.querySelector('#terminal-input');
        }

        /**
         * Setup event listeners
         * @private
         */
        _setupEventListeners() {
            this.inputElement.addEventListener('keydown', (e) => {
                if (e.key === 'Enter') {
                    this._handleCommandSubmit();
                } else if (e.key === 'ArrowUp') {
                    e.preventDefault();
                    this._navigateHistory(-1);
                } else if (e.key === 'ArrowDown') {
                    e.preventDefault();
                    this._navigateHistory(1);
                } else if (e.key === 'Tab') {
                    e.preventDefault();
                    this._handleAutoComplete();
                }
            });

            // Listen for command output
            this.commandManager.sdk.on('commandOutput', ({ execution, output }) => {
                if (execution === this.currentExecution) {
                    this._appendOutput(output.content, output.type);
                }
            });

            // Listen for command completion
            this.commandManager.sdk.on('commandComplete', ({ execution }) => {
                if (execution === this.currentExecution) {
                    this._appendOutput(
                        `\nCommand completed with exit code: ${execution.exitCode}`,
                        'system'
                    );
                    this.currentExecution = null;
                    this.inputElement.disabled = false;
                    this.inputElement.focus();
                }
            });

            // Listen for command errors
            this.commandManager.sdk.on('commandError', ({ execution, error }) => {
                if (execution === this.currentExecution) {
                    this._appendOutput(`\nError: ${error.message}`, 'error');
                    this.currentExecution = null;
                    this.inputElement.disabled = false;
                    this.inputElement.focus();
                }
            });
        }

        /**
         * Handle command submit
         * @private
         */
        async _handleCommandSubmit() {
            const command = this.inputElement.value.trim();

            if (!command) {
                return;
            }

            // Display command in output
            this._appendOutput(`$ ${command}`, 'command');

            // Clear input
            this.inputElement.value = '';
            this.historyIndex = -1;

            // Handle built-in commands
            if (command === 'clear') {
                this.outputElement.innerHTML = '';
                return;
            }

            if (command === 'history') {
                const history = this.commandManager.getHistory();
                history.forEach((cmd, i) => {
                    this._appendOutput(`${i + 1}  ${cmd}`, 'output');
                });
                return;
            }

            // Execute command
            try {
                this.inputElement.disabled = true;

                // Get peer ID (should be set externally)
                const peerId = this.peerId || 'default';

                this.currentExecution = await this.commandManager.executeCommand(
                    command,
                    peerId
                );
            } catch (error) {
                this._appendOutput(`Error: ${error.message}`, 'error');
                this.inputElement.disabled = false;
            }
        }

        /**
         * Append output to terminal
         * @private
         */
        _appendOutput(content, type = 'output') {
            const line = document.createElement('div');
            line.className = `terminal-line terminal-${type}`;
            line.textContent = content;
            this.outputElement.appendChild(line);

            // Auto-scroll to bottom
            this.outputElement.scrollTop = this.outputElement.scrollHeight;
        }

        /**
         * Navigate command history
         * @private
         */
        _navigateHistory(direction) {
            const history = this.commandManager.getHistory();

            if (history.length === 0) {
                return;
            }

            this.historyIndex += direction;

            if (this.historyIndex < 0) {
                this.historyIndex = -1;
                this.inputElement.value = '';
            } else if (this.historyIndex >= history.length) {
                this.historyIndex = history.length - 1;
            }

            if (this.historyIndex >= 0) {
                this.inputElement.value = history[history.length - 1 - this.historyIndex];
            }
        }

        /**
         * Handle auto-complete
         * @private
         */
        _handleAutoComplete() {
            // Basic auto-complete from history
            const input = this.inputElement.value;

            if (!input) {
                return;
            }

            const matches = this.commandManager.searchHistory(input);

            if (matches.length > 0) {
                this.inputElement.value = matches[matches.length - 1];
            }
        }

        /**
         * Set target peer ID
         * @param {string} peerId - Peer ID
         */
        setPeerId(peerId) {
            this.peerId = peerId;
        }

        /**
         * Clear terminal
         */
        clear() {
            this.outputElement.innerHTML = '';
        }

        /**
         * Focus input
         */
        focus() {
            this.inputElement.focus();
        }
    }

    // Export
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = { CommandExecutionManager, TerminalInterface };
    } else {
        global.KizunaCommand = CommandExecutionManager;
        global.KizunaTerminal = TerminalInterface;
    }

})(typeof window !== 'undefined' ? window : this);
