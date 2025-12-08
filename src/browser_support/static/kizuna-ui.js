/**
 * Kizuna UI Components
 * 
 * Responsive web UI components for file transfer, video streaming, 
 * command terminal, and peer management.
 * 
 * @version 1.0.0
 */

(function (global) {
    'use strict';

    /**
     * File Transfer UI Component
     */
    class FileTransferUI {
        constructor(container, sdk) {
            this.container = container;
            this.sdk = sdk;
            this.transfers = new Map();
            this.dragCounter = 0;
            this.init();
        }

        init() {
            this.render();
            this.setupEventListeners();
            this.setupSDKListeners();
        }

        render() {
            this.container.innerHTML = `
                <div class="file-transfer-ui">
                    <div class="file-transfer-header">
                        <h2>File Transfer</h2>
                        <div class="transfer-stats">
                            <span class="stat">
                                <span class="stat-label">Active:</span>
                                <span class="stat-value" id="active-transfers">0</span>
                            </span>
                            <span class="stat">
                                <span class="stat-label">Queued:</span>
                                <span class="stat-value" id="queued-transfers">0</span>
                            </span>
                        </div>
                    </div>

                    <div class="drop-zone" id="drop-zone">
                        <div class="drop-zone-content">
                            <svg class="drop-zone-icon" viewBox="0 0 24 24" width="48" height="48">
                                <path fill="currentColor" d="M19.35 10.04C18.67 6.59 15.64 4 12 4 9.11 4 6.6 5.64 5.35 8.04 2.34 8.36 0 10.91 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96zM14 13v4h-4v-4H7l5-5 5 5h-3z"/>
                            </svg>
                            <p class="drop-zone-text">Drag and drop files here</p>
                            <p class="drop-zone-subtext">or</p>
                            <button class="btn btn-primary" id="select-files-btn">Select Files</button>
                            <input type="file" id="file-input" multiple hidden>
                        </div>
                    </div>

                    <div class="transfer-queue" id="transfer-queue">
                        <h3>Transfer Queue</h3>
                        <div class="queue-list" id="queue-list">
                            <p class="empty-state">No transfers in progress</p>
                        </div>
                    </div>
                </div>
            `;
        }

        setupEventListeners() {
            const dropZone = this.container.querySelector('#drop-zone');
            const fileInput = this.container.querySelector('#file-input');
            const selectBtn = this.container.querySelector('#select-files-btn');

            // Drag and drop handlers
            dropZone.addEventListener('dragenter', (e) => {
                e.preventDefault();
                e.stopPropagation();
                this.dragCounter++;
                dropZone.classList.add('drag-over');
            });

            dropZone.addEventListener('dragleave', (e) => {
                e.preventDefault();
                e.stopPropagation();
                this.dragCounter--;
                if (this.dragCounter === 0) {
                    dropZone.classList.remove('drag-over');
                }
            });

            dropZone.addEventListener('dragover', (e) => {
                e.preventDefault();
                e.stopPropagation();
            });

            dropZone.addEventListener('drop', (e) => {
                e.preventDefault();
                e.stopPropagation();
                this.dragCounter = 0;
                dropZone.classList.remove('drag-over');

                const files = Array.from(e.dataTransfer.files);
                this.handleFiles(files);
            });

            // File selection handler
            selectBtn.addEventListener('click', () => {
                fileInput.click();
            });

            fileInput.addEventListener('change', (e) => {
                const files = Array.from(e.target.files);
                this.handleFiles(files);
                fileInput.value = ''; // Reset input
            });
        }

        setupSDKListeners() {
            this.sdk.on('transferProgress', (data) => {
                this.updateTransferProgress(data.transferId, data.progress);
            });

            this.sdk.on('transferComplete', (data) => {
                this.completeTransfer(data.transferId);
            });

            this.sdk.on('transferError', (data) => {
                this.errorTransfer(data.transferId, data.error);
            });
        }

        async handleFiles(files) {
            if (files.length === 0) return;

            for (const file of files) {
                await this.addTransfer(file);
            }
        }

        async addTransfer(file) {
            const transferId = this._generateId();
            const transfer = {
                id: transferId,
                file: file,
                name: file.name,
                size: file.size,
                progress: 0,
                status: 'queued',
                startTime: Date.now()
            };

            this.transfers.set(transferId, transfer);
            this.renderTransfer(transfer);
            this.updateStats();

            try {
                transfer.status = 'uploading';
                this.updateTransferStatus(transferId, 'uploading');

                await this.sdk.uploadFile(file, (progress) => {
                    this.updateTransferProgress(transferId, progress);
                });

                transfer.status = 'completed';
                this.completeTransfer(transferId);
            } catch (error) {
                transfer.status = 'error';
                this.errorTransfer(transferId, error.message);
            }
        }

        renderTransfer(transfer) {
            const queueList = this.container.querySelector('#queue-list');
            const emptyState = queueList.querySelector('.empty-state');
            if (emptyState) {
                emptyState.remove();
            }

            const transferEl = document.createElement('div');
            transferEl.className = 'transfer-item';
            transferEl.id = `transfer-${transfer.id}`;
            transferEl.innerHTML = `
                <div class="transfer-info">
                    <div class="transfer-icon">
                        <svg viewBox="0 0 24 24" width="24" height="24">
                            <path fill="currentColor" d="M14,2H6A2,2 0 0,0 4,4V20A2,2 0 0,0 6,22H18A2,2 0 0,0 20,20V8L14,2M18,20H6V4H13V9H18V20Z"/>
                        </svg>
                    </div>
                    <div class="transfer-details">
                        <div class="transfer-name">${this._escapeHtml(transfer.name)}</div>
                        <div class="transfer-size">${this._formatBytes(transfer.size)}</div>
                    </div>
                </div>
                <div class="transfer-progress-container">
                    <div class="transfer-progress-bar">
                        <div class="transfer-progress-fill" style="width: ${transfer.progress}%"></div>
                    </div>
                    <div class="transfer-status">
                        <span class="status-text">${transfer.status}</span>
                        <span class="status-percent">${transfer.progress}%</span>
                    </div>
                </div>
                <div class="transfer-actions">
                    <button class="btn-icon cancel-btn" data-transfer-id="${transfer.id}" title="Cancel">
                        <svg viewBox="0 0 24 24" width="20" height="20">
                            <path fill="currentColor" d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/>
                        </svg>
                    </button>
                </div>
            `;

            queueList.appendChild(transferEl);

            // Setup cancel button
            const cancelBtn = transferEl.querySelector('.cancel-btn');
            cancelBtn.addEventListener('click', () => {
                this.cancelTransfer(transfer.id);
            });
        }

        updateTransferProgress(transferId, progress) {
            const transfer = this.transfers.get(transferId);
            if (!transfer) return;

            transfer.progress = Math.round(progress);
            const transferEl = this.container.querySelector(`#transfer-${transferId}`);
            if (transferEl) {
                const progressFill = transferEl.querySelector('.transfer-progress-fill');
                const statusPercent = transferEl.querySelector('.status-percent');
                progressFill.style.width = `${transfer.progress}%`;
                statusPercent.textContent = `${transfer.progress}%`;
            }
        }

        updateTransferStatus(transferId, status) {
            const transfer = this.transfers.get(transferId);
            if (!transfer) return;

            transfer.status = status;
            const transferEl = this.container.querySelector(`#transfer-${transferId}`);
            if (transferEl) {
                const statusText = transferEl.querySelector('.status-text');
                statusText.textContent = status;
                transferEl.className = `transfer-item transfer-${status}`;
            }
            this.updateStats();
        }

        completeTransfer(transferId) {
            this.updateTransferStatus(transferId, 'completed');
            setTimeout(() => {
                this.removeTransfer(transferId);
            }, 3000);
        }

        errorTransfer(transferId, errorMessage) {
            const transfer = this.transfers.get(transferId);
            if (!transfer) return;

            transfer.status = 'error';
            transfer.error = errorMessage;

            const transferEl = this.container.querySelector(`#transfer-${transferId}`);
            if (transferEl) {
                const statusText = transferEl.querySelector('.status-text');
                statusText.textContent = `Error: ${errorMessage}`;
                transferEl.className = 'transfer-item transfer-error';
            }
            this.updateStats();
        }

        cancelTransfer(transferId) {
            this.removeTransfer(transferId);
            // TODO: Notify SDK to cancel transfer
        }

        removeTransfer(transferId) {
            this.transfers.delete(transferId);
            const transferEl = this.container.querySelector(`#transfer-${transferId}`);
            if (transferEl) {
                transferEl.remove();
            }

            const queueList = this.container.querySelector('#queue-list');
            if (queueList.children.length === 0) {
                queueList.innerHTML = '<p class="empty-state">No transfers in progress</p>';
            }

            this.updateStats();
        }

        updateStats() {
            let active = 0;
            let queued = 0;

            this.transfers.forEach(transfer => {
                if (transfer.status === 'uploading' || transfer.status === 'downloading') {
                    active++;
                } else if (transfer.status === 'queued') {
                    queued++;
                }
            });

            this.container.querySelector('#active-transfers').textContent = active;
            this.container.querySelector('#queued-transfers').textContent = queued;
        }

        _formatBytes(bytes) {
            if (bytes === 0) return '0 Bytes';
            const k = 1024;
            const sizes = ['Bytes', 'KB', 'MB', 'GB'];
            const i = Math.floor(Math.log(bytes) / Math.log(k));
            return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i];
        }

        _escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }

        _generateId() {
            return 'transfer-' + Date.now() + '-' + Math.random().toString(36).substr(2, 9);
        }
    }


    /**
     * Video Streaming Player UI Component
     */
    class VideoPlayerUI {
        constructor(container, sdk) {
            this.container = container;
            this.sdk = sdk;
            this.stream = null;
            this.isFullscreen = false;
            this.quality = 'auto';
            this.init();
        }

        init() {
            this.render();
            this.setupEventListeners();
            this.setupSDKListeners();
        }

        render() {
            this.container.innerHTML = `
                <div class="video-player-ui">
                    <div class="video-player-header">
                        <h2>Video Stream</h2>
                        <div class="connection-status">
                            <span class="status-indicator" id="stream-status"></span>
                            <span class="status-text" id="stream-status-text">Disconnected</span>
                        </div>
                    </div>

                    <div class="video-container" id="video-container">
                        <video id="video-element" autoplay playsinline></video>
                        
                        <div class="video-overlay" id="video-overlay">
                            <div class="video-placeholder">
                                <svg class="video-icon" viewBox="0 0 24 24" width="64" height="64">
                                    <path fill="currentColor" d="M17,10.5V7A1,1 0 0,0 16,6H4A1,1 0 0,0 3,7V17A1,1 0 0,0 4,18H16A1,1 0 0,0 17,17V13.5L21,17.5V6.5L17,10.5Z"/>
                                </svg>
                                <p>No video stream</p>
                                <button class="btn btn-primary" id="start-stream-btn">Start Stream</button>
                            </div>
                        </div>

                        <div class="video-controls" id="video-controls">
                            <div class="controls-left">
                                <button class="btn-icon" id="play-pause-btn" title="Play/Pause">
                                    <svg class="play-icon" viewBox="0 0 24 24" width="24" height="24">
                                        <path fill="currentColor" d="M8,5.14V19.14L19,12.14L8,5.14Z"/>
                                    </svg>
                                    <svg class="pause-icon hidden" viewBox="0 0 24 24" width="24" height="24">
                                        <path fill="currentColor" d="M14,19H18V5H14M6,19H10V5H6V19Z"/>
                                    </svg>
                                </button>
                                <button class="btn-icon" id="volume-btn" title="Volume">
                                    <svg viewBox="0 0 24 24" width="24" height="24">
                                        <path fill="currentColor" d="M14,3.23V5.29C16.89,6.15 19,8.83 19,12C19,15.17 16.89,17.84 14,18.7V20.77C18,19.86 21,16.28 21,12C21,7.72 18,4.14 14,3.23M16.5,12C16.5,10.23 15.5,8.71 14,7.97V16C15.5,15.29 16.5,13.76 16.5,12M3,9V15H7L12,20V4L7,9H3Z"/>
                                    </svg>
                                </button>
                            </div>

                            <div class="controls-center">
                                <div class="quality-selector">
                                    <label>Quality:</label>
                                    <select id="quality-select">
                                        <option value="auto">Auto</option>
                                        <option value="high">High</option>
                                        <option value="medium">Medium</option>
                                        <option value="low">Low</option>
                                    </select>
                                </div>
                            </div>

                            <div class="controls-right">
                                <button class="btn-icon" id="fullscreen-btn" title="Fullscreen">
                                    <svg class="fullscreen-icon" viewBox="0 0 24 24" width="24" height="24">
                                        <path fill="currentColor" d="M5,5H10V7H7V10H5V5M14,5H19V10H17V7H14V5M17,14H19V19H14V17H17V14M10,17V19H5V14H7V17H10Z"/>
                                    </svg>
                                    <svg class="exit-fullscreen-icon hidden" viewBox="0 0 24 24" width="24" height="24">
                                        <path fill="currentColor" d="M14,14H19V16H16V19H14V14M5,14H10V19H8V16H5V14M8,5H10V10H5V8H8V5M19,8V10H14V5H16V8H19Z"/>
                                    </svg>
                                </button>
                            </div>
                        </div>
                    </div>

                    <div class="stream-info" id="stream-info">
                        <div class="info-item">
                            <span class="info-label">Resolution:</span>
                            <span class="info-value" id="resolution">-</span>
                        </div>
                        <div class="info-item">
                            <span class="info-label">Bitrate:</span>
                            <span class="info-value" id="bitrate">-</span>
                        </div>
                        <div class="info-item">
                            <span class="info-label">Latency:</span>
                            <span class="info-value" id="latency">-</span>
                        </div>
                    </div>
                </div>
            `;
        }

        setupEventListeners() {
            const videoElement = this.container.querySelector('#video-element');
            const startStreamBtn = this.container.querySelector('#start-stream-btn');
            const playPauseBtn = this.container.querySelector('#play-pause-btn');
            const volumeBtn = this.container.querySelector('#volume-btn');
            const fullscreenBtn = this.container.querySelector('#fullscreen-btn');
            const qualitySelect = this.container.querySelector('#quality-select');
            const videoContainer = this.container.querySelector('#video-container');

            // Start stream
            startStreamBtn.addEventListener('click', () => {
                this.startStream();
            });

            // Play/Pause
            playPauseBtn.addEventListener('click', () => {
                if (videoElement.paused) {
                    videoElement.play();
                } else {
                    videoElement.pause();
                }
            });

            videoElement.addEventListener('play', () => {
                playPauseBtn.querySelector('.play-icon').classList.add('hidden');
                playPauseBtn.querySelector('.pause-icon').classList.remove('hidden');
            });

            videoElement.addEventListener('pause', () => {
                playPauseBtn.querySelector('.play-icon').classList.remove('hidden');
                playPauseBtn.querySelector('.pause-icon').classList.add('hidden');
            });

            // Volume
            volumeBtn.addEventListener('click', () => {
                videoElement.muted = !videoElement.muted;
            });

            // Quality selection
            qualitySelect.addEventListener('change', (e) => {
                this.changeQuality(e.target.value);
            });

            // Fullscreen
            fullscreenBtn.addEventListener('click', () => {
                this.toggleFullscreen();
            });

            document.addEventListener('fullscreenchange', () => {
                this.isFullscreen = !!document.fullscreenElement;
                this.updateFullscreenButton();
            });

            // Show/hide controls on hover
            videoContainer.addEventListener('mouseenter', () => {
                if (this.stream) {
                    this.container.querySelector('#video-controls').classList.add('visible');
                }
            });

            videoContainer.addEventListener('mouseleave', () => {
                this.container.querySelector('#video-controls').classList.remove('visible');
            });
        }

        setupSDKListeners() {
            this.sdk.on('streamStarted', (data) => {
                this.onStreamStarted(data);
            });

            this.sdk.on('streamEnded', () => {
                this.onStreamEnded();
            });

            this.sdk.on('streamStats', (data) => {
                this.updateStreamStats(data);
            });
        }

        async startStream() {
            try {
                const stream = await this.sdk.requestVideoStream();
                this.setStream(stream);
            } catch (error) {
                console.error('Failed to start stream:', error);
                this.showError('Failed to start video stream');
            }
        }

        setStream(stream) {
            this.stream = stream;
            const videoElement = this.container.querySelector('#video-element');
            videoElement.srcObject = stream;

            this.container.querySelector('#video-overlay').classList.add('hidden');
            this.container.querySelector('#video-controls').classList.add('visible');
            this.updateStatus('connected', 'Connected');
        }

        onStreamStarted(data) {
            this.updateStatus('streaming', 'Streaming');
        }

        onStreamEnded() {
            this.stream = null;
            const videoElement = this.container.querySelector('#video-element');
            videoElement.srcObject = null;

            this.container.querySelector('#video-overlay').classList.remove('hidden');
            this.container.querySelector('#video-controls').classList.remove('visible');
            this.updateStatus('disconnected', 'Disconnected');
        }

        changeQuality(quality) {
            this.quality = quality;
            // TODO: Notify SDK to change quality
            this.sdk.setVideoQuality(quality);
        }

        toggleFullscreen() {
            const videoContainer = this.container.querySelector('#video-container');

            if (!this.isFullscreen) {
                if (videoContainer.requestFullscreen) {
                    videoContainer.requestFullscreen();
                } else if (videoContainer.webkitRequestFullscreen) {
                    videoContainer.webkitRequestFullscreen();
                } else if (videoContainer.mozRequestFullScreen) {
                    videoContainer.mozRequestFullScreen();
                }
            } else {
                if (document.exitFullscreen) {
                    document.exitFullscreen();
                } else if (document.webkitExitFullscreen) {
                    document.webkitExitFullscreen();
                } else if (document.mozCancelFullScreen) {
                    document.mozCancelFullScreen();
                }
            }
        }

        updateFullscreenButton() {
            const fullscreenBtn = this.container.querySelector('#fullscreen-btn');
            if (this.isFullscreen) {
                fullscreenBtn.querySelector('.fullscreen-icon').classList.add('hidden');
                fullscreenBtn.querySelector('.exit-fullscreen-icon').classList.remove('hidden');
            } else {
                fullscreenBtn.querySelector('.fullscreen-icon').classList.remove('hidden');
                fullscreenBtn.querySelector('.exit-fullscreen-icon').classList.add('hidden');
            }
        }

        updateStatus(status, text) {
            const statusIndicator = this.container.querySelector('#stream-status');
            const statusText = this.container.querySelector('#stream-status-text');

            statusIndicator.className = `status-indicator status-${status}`;
            statusText.textContent = text;
        }

        updateStreamStats(stats) {
            this.container.querySelector('#resolution').textContent =
                `${stats.width}x${stats.height}`;
            this.container.querySelector('#bitrate').textContent =
                `${Math.round(stats.bitrate / 1000)} kbps`;
            this.container.querySelector('#latency').textContent =
                `${stats.latency} ms`;
        }

        showError(message) {
            // TODO: Show error notification
            console.error(message);
        }
    }


    /**
     * Command Terminal UI Component
     */
    class CommandTerminalUI {
        constructor(container, sdk) {
            this.container = container;
            this.sdk = sdk;
            this.history = [];
            this.historyIndex = -1;
            this.savedTemplates = [];
            this.currentCommand = '';
            this.init();
        }

        init() {
            this.render();
            this.setupEventListeners();
            this.setupSDKListeners();
            this.loadHistory();
            this.loadTemplates();
        }

        render() {
            this.container.innerHTML = `
                <div class="command-terminal-ui">
                    <div class="terminal-header">
                        <h2>Command Terminal</h2>
                        <div class="terminal-actions">
                            <button class="btn-icon" id="clear-terminal-btn" title="Clear">
                                <svg viewBox="0 0 24 24" width="20" height="20">
                                    <path fill="currentColor" d="M19,4H15.5L14.5,3H9.5L8.5,4H5V6H19M6,19A2,2 0 0,0 8,21H16A2,2 0 0,0 18,19V7H6V19Z"/>
                                </svg>
                            </button>
                            <button class="btn-icon" id="templates-btn" title="Templates">
                                <svg viewBox="0 0 24 24" width="20" height="20">
                                    <path fill="currentColor" d="M19,3H5C3.89,3 3,3.89 3,5V19A2,2 0 0,0 5,21H19A2,2 0 0,0 21,19V5C21,3.89 20.1,3 19,3M19,5V19H5V5H19Z"/>
                                </svg>
                            </button>
                        </div>
                    </div>

                    <div class="terminal-output" id="terminal-output">
                        <div class="terminal-welcome">
                            <p>Kizuna Command Terminal</p>
                            <p>Type 'help' for available commands</p>
                        </div>
                    </div>

                    <div class="terminal-input-container">
                        <div class="terminal-prompt">$</div>
                        <input 
                            type="text" 
                            class="terminal-input" 
                            id="terminal-input" 
                            placeholder="Enter command..."
                            autocomplete="off"
                        >
                        <button class="btn btn-primary" id="execute-btn">Execute</button>
                    </div>

                    <div class="terminal-suggestions" id="terminal-suggestions"></div>

                    <div class="templates-panel hidden" id="templates-panel">
                        <div class="templates-header">
                            <h3>Command Templates</h3>
                            <button class="btn-icon" id="close-templates-btn">
                                <svg viewBox="0 0 24 24" width="20" height="20">
                                    <path fill="currentColor" d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/>
                                </svg>
                            </button>
                        </div>
                        <div class="templates-list" id="templates-list">
                            <p class="empty-state">No saved templates</p>
                        </div>
                        <div class="templates-actions">
                            <input 
                                type="text" 
                                id="template-name-input" 
                                placeholder="Template name"
                            >
                            <button class="btn btn-secondary" id="save-template-btn">Save Current</button>
                        </div>
                    </div>
                </div>
            `;
        }

        setupEventListeners() {
            const terminalInput = this.container.querySelector('#terminal-input');
            const executeBtn = this.container.querySelector('#execute-btn');
            const clearBtn = this.container.querySelector('#clear-terminal-btn');
            const templatesBtn = this.container.querySelector('#templates-btn');
            const closeTemplatesBtn = this.container.querySelector('#close-templates-btn');
            const saveTemplateBtn = this.container.querySelector('#save-template-btn');

            // Execute command
            executeBtn.addEventListener('click', () => {
                this.executeCommand();
            });

            terminalInput.addEventListener('keydown', (e) => {
                if (e.key === 'Enter') {
                    this.executeCommand();
                } else if (e.key === 'ArrowUp') {
                    e.preventDefault();
                    this.navigateHistory('up');
                } else if (e.key === 'ArrowDown') {
                    e.preventDefault();
                    this.navigateHistory('down');
                } else if (e.key === 'Tab') {
                    e.preventDefault();
                    this.autoComplete();
                }
            });

            // Auto-complete suggestions
            terminalInput.addEventListener('input', (e) => {
                this.showSuggestions(e.target.value);
            });

            // Clear terminal
            clearBtn.addEventListener('click', () => {
                this.clearOutput();
            });

            // Templates panel
            templatesBtn.addEventListener('click', () => {
                this.toggleTemplatesPanel();
            });

            closeTemplatesBtn.addEventListener('click', () => {
                this.toggleTemplatesPanel();
            });

            saveTemplateBtn.addEventListener('click', () => {
                this.saveTemplate();
            });
        }

        setupSDKListeners() {
            this.sdk.on('commandOutput', (data) => {
                this.appendOutput(data.output, 'output');
            });

            this.sdk.on('commandError', (data) => {
                this.appendOutput(data.error, 'error');
            });

            this.sdk.on('commandComplete', (data) => {
                this.appendOutput(`Command completed with exit code: ${data.exitCode}`, 'info');
            });
        }

        async executeCommand() {
            const terminalInput = this.container.querySelector('#terminal-input');
            const command = terminalInput.value.trim();

            if (!command) return;

            // Add to history
            this.history.push(command);
            this.historyIndex = this.history.length;
            this.saveHistory();

            // Display command
            this.appendOutput(`$ ${command}`, 'command');

            // Clear input
            terminalInput.value = '';
            this.hideSuggestions();

            // Execute command
            try {
                await this.sdk.executeCommand(command);
            } catch (error) {
                this.appendOutput(`Error: ${error.message}`, 'error');
            }
        }

        appendOutput(text, type = 'output') {
            const terminalOutput = this.container.querySelector('#terminal-output');
            const outputLine = document.createElement('div');
            outputLine.className = `terminal-line terminal-${type}`;
            outputLine.textContent = text;
            terminalOutput.appendChild(outputLine);

            // Auto-scroll to bottom
            terminalOutput.scrollTop = terminalOutput.scrollHeight;
        }

        clearOutput() {
            const terminalOutput = this.container.querySelector('#terminal-output');
            terminalOutput.innerHTML = `
                <div class="terminal-welcome">
                    <p>Terminal cleared</p>
                </div>
            `;
        }

        navigateHistory(direction) {
            if (this.history.length === 0) return;

            const terminalInput = this.container.querySelector('#terminal-input');

            if (direction === 'up') {
                if (this.historyIndex > 0) {
                    this.historyIndex--;
                    terminalInput.value = this.history[this.historyIndex];
                }
            } else if (direction === 'down') {
                if (this.historyIndex < this.history.length - 1) {
                    this.historyIndex++;
                    terminalInput.value = this.history[this.historyIndex];
                } else {
                    this.historyIndex = this.history.length;
                    terminalInput.value = '';
                }
            }
        }

        showSuggestions(input) {
            if (!input) {
                this.hideSuggestions();
                return;
            }

            // Basic command suggestions
            const commands = ['ls', 'cd', 'pwd', 'cat', 'echo', 'mkdir', 'rm', 'cp', 'mv', 'help'];
            const suggestions = commands.filter(cmd => cmd.startsWith(input));

            if (suggestions.length === 0) {
                this.hideSuggestions();
                return;
            }

            const suggestionsEl = this.container.querySelector('#terminal-suggestions');
            suggestionsEl.innerHTML = suggestions.map(cmd =>
                `<div class="suggestion-item" data-command="${cmd}">${cmd}</div>`
            ).join('');
            suggestionsEl.classList.remove('hidden');

            // Click handler for suggestions
            suggestionsEl.querySelectorAll('.suggestion-item').forEach(item => {
                item.addEventListener('click', () => {
                    const terminalInput = this.container.querySelector('#terminal-input');
                    terminalInput.value = item.dataset.command;
                    this.hideSuggestions();
                    terminalInput.focus();
                });
            });
        }

        hideSuggestions() {
            const suggestionsEl = this.container.querySelector('#terminal-suggestions');
            suggestionsEl.classList.add('hidden');
            suggestionsEl.innerHTML = '';
        }

        autoComplete() {
            const terminalInput = this.container.querySelector('#terminal-input');
            const input = terminalInput.value;

            const commands = ['ls', 'cd', 'pwd', 'cat', 'echo', 'mkdir', 'rm', 'cp', 'mv', 'help'];
            const match = commands.find(cmd => cmd.startsWith(input));

            if (match) {
                terminalInput.value = match;
            }
        }

        toggleTemplatesPanel() {
            const panel = this.container.querySelector('#templates-panel');
            panel.classList.toggle('hidden');

            if (!panel.classList.contains('hidden')) {
                this.renderTemplates();
            }
        }

        renderTemplates() {
            const templatesList = this.container.querySelector('#templates-list');

            if (this.savedTemplates.length === 0) {
                templatesList.innerHTML = '<p class="empty-state">No saved templates</p>';
                return;
            }

            templatesList.innerHTML = this.savedTemplates.map((template, index) => `
                <div class="template-item">
                    <div class="template-info">
                        <div class="template-name">${this._escapeHtml(template.name)}</div>
                        <div class="template-command">${this._escapeHtml(template.command)}</div>
                    </div>
                    <div class="template-actions">
                        <button class="btn-icon use-template-btn" data-index="${index}" title="Use">
                            <svg viewBox="0 0 24 24" width="18" height="18">
                                <path fill="currentColor" d="M8.59,16.58L13.17,12L8.59,7.41L10,6L16,12L10,18L8.59,16.58Z"/>
                            </svg>
                        </button>
                        <button class="btn-icon delete-template-btn" data-index="${index}" title="Delete">
                            <svg viewBox="0 0 24 24" width="18" height="18">
                                <path fill="currentColor" d="M19,4H15.5L14.5,3H9.5L8.5,4H5V6H19M6,19A2,2 0 0,0 8,21H16A2,2 0 0,0 18,19V7H6V19Z"/>
                            </svg>
                        </button>
                    </div>
                </div>
            `).join('');

            // Setup event listeners
            templatesList.querySelectorAll('.use-template-btn').forEach(btn => {
                btn.addEventListener('click', () => {
                    const index = parseInt(btn.dataset.index);
                    this.useTemplate(index);
                });
            });

            templatesList.querySelectorAll('.delete-template-btn').forEach(btn => {
                btn.addEventListener('click', () => {
                    const index = parseInt(btn.dataset.index);
                    this.deleteTemplate(index);
                });
            });
        }

        saveTemplate() {
            const terminalInput = this.container.querySelector('#terminal-input');
            const templateNameInput = this.container.querySelector('#template-name-input');

            const command = terminalInput.value.trim();
            const name = templateNameInput.value.trim();

            if (!command || !name) {
                alert('Please enter both template name and command');
                return;
            }

            this.savedTemplates.push({ name, command });
            this.saveTemplates();
            this.renderTemplates();

            templateNameInput.value = '';
        }

        useTemplate(index) {
            const template = this.savedTemplates[index];
            if (template) {
                const terminalInput = this.container.querySelector('#terminal-input');
                terminalInput.value = template.command;
                this.toggleTemplatesPanel();
                terminalInput.focus();
            }
        }

        deleteTemplate(index) {
            this.savedTemplates.splice(index, 1);
            this.saveTemplates();
            this.renderTemplates();
        }

        loadHistory() {
            const saved = localStorage.getItem('kizuna-terminal-history');
            if (saved) {
                this.history = JSON.parse(saved);
                this.historyIndex = this.history.length;
            }
        }

        saveHistory() {
            localStorage.setItem('kizuna-terminal-history', JSON.stringify(this.history));
        }

        loadTemplates() {
            const saved = localStorage.getItem('kizuna-terminal-templates');
            if (saved) {
                this.savedTemplates = JSON.parse(saved);
            }
        }

        saveTemplates() {
            localStorage.setItem('kizuna-terminal-templates', JSON.stringify(this.savedTemplates));
        }

        _escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }
    }


    /**
     * Peer Management UI Component
     */
    class PeerManagementUI {
        constructor(container, sdk) {
            this.container = container;
            this.sdk = sdk;
            this.peers = new Map();
            this.selectedPeer = null;
            this.init();
        }

        init() {
            this.render();
            this.setupEventListeners();
            this.setupSDKListeners();
            this.discoverPeers();
        }

        render() {
            this.container.innerHTML = `
                <div class="peer-management-ui">
                    <div class="peer-header">
                        <h2>Peers</h2>
                        <button class="btn btn-primary" id="discover-peers-btn">
                            <svg viewBox="0 0 24 24" width="18" height="18">
                                <path fill="currentColor" d="M9.5,3A6.5,6.5 0 0,1 16,9.5C16,11.11 15.41,12.59 14.44,13.73L14.71,14H15.5L20.5,19L19,20.5L14,15.5V14.71L13.73,14.44C12.59,15.41 11.11,16 9.5,16A6.5,6.5 0 0,1 3,9.5A6.5,6.5 0 0,1 9.5,3M9.5,5C7,5 5,7 5,9.5C5,12 7,14 9.5,14C12,14 14,12 14,9.5C14,7 12,5 9.5,5Z"/>
                            </svg>
                            Discover
                        </button>
                    </div>

                    <div class="peer-list" id="peer-list">
                        <div class="peer-list-loading">
                            <div class="spinner"></div>
                            <p>Discovering peers...</p>
                        </div>
                    </div>

                    <div class="peer-details hidden" id="peer-details">
                        <div class="peer-details-header">
                            <h3 id="peer-details-name">Peer Details</h3>
                            <button class="btn-icon" id="close-details-btn">
                                <svg viewBox="0 0 24 24" width="20" height="20">
                                    <path fill="currentColor" d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/>
                                </svg>
                            </button>
                        </div>

                        <div class="peer-details-content">
                            <div class="detail-section">
                                <h4>Connection</h4>
                                <div class="detail-item">
                                    <span class="detail-label">Status:</span>
                                    <span class="detail-value" id="peer-status">-</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label">Peer ID:</span>
                                    <span class="detail-value" id="peer-id">-</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label">Device Type:</span>
                                    <span class="detail-value" id="peer-device-type">-</span>
                                </div>
                            </div>

                            <div class="detail-section">
                                <h4>Connection Quality</h4>
                                <div class="quality-indicator">
                                    <div class="quality-bar">
                                        <div class="quality-fill" id="quality-fill"></div>
                                    </div>
                                    <span class="quality-text" id="quality-text">-</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label">Latency:</span>
                                    <span class="detail-value" id="peer-latency">-</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label">Bandwidth:</span>
                                    <span class="detail-value" id="peer-bandwidth">-</span>
                                </div>
                            </div>

                            <div class="detail-section">
                                <h4>Capabilities</h4>
                                <div class="capabilities-list" id="capabilities-list">
                                    <span class="capability-badge">Loading...</span>
                                </div>
                            </div>

                            <div class="detail-section">
                                <h4>Actions</h4>
                                <div class="peer-actions">
                                    <button class="btn btn-primary" id="connect-peer-btn">Connect</button>
                                    <button class="btn btn-secondary" id="disconnect-peer-btn">Disconnect</button>
                                    <button class="btn btn-secondary" id="test-connection-btn">Test Connection</button>
                                </div>
                            </div>

                            <div class="detail-section">
                                <h4>Troubleshooting</h4>
                                <div class="troubleshooting-info" id="troubleshooting-info">
                                    <p>Connection diagnostics will appear here</p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            `;
        }

        setupEventListeners() {
            const discoverBtn = this.container.querySelector('#discover-peers-btn');
            const closeDetailsBtn = this.container.querySelector('#close-details-btn');
            const connectBtn = this.container.querySelector('#connect-peer-btn');
            const disconnectBtn = this.container.querySelector('#disconnect-peer-btn');
            const testConnectionBtn = this.container.querySelector('#test-connection-btn');

            discoverBtn.addEventListener('click', () => {
                this.discoverPeers();
            });

            closeDetailsBtn.addEventListener('click', () => {
                this.hideDetails();
            });

            connectBtn.addEventListener('click', () => {
                if (this.selectedPeer) {
                    this.connectToPeer(this.selectedPeer);
                }
            });

            disconnectBtn.addEventListener('click', () => {
                if (this.selectedPeer) {
                    this.disconnectFromPeer(this.selectedPeer);
                }
            });

            testConnectionBtn.addEventListener('click', () => {
                if (this.selectedPeer) {
                    this.testConnection(this.selectedPeer);
                }
            });
        }

        setupSDKListeners() {
            this.sdk.on('peerDiscovered', (data) => {
                this.addPeer(data.peer);
            });

            this.sdk.on('peerConnected', (data) => {
                this.updatePeerStatus(data.peerId, 'connected');
            });

            this.sdk.on('peerDisconnected', (data) => {
                this.updatePeerStatus(data.peerId, 'disconnected');
            });

            this.sdk.on('peerStats', (data) => {
                this.updatePeerStats(data.peerId, data.stats);
            });
        }

        async discoverPeers() {
            const peerList = this.container.querySelector('#peer-list');
            peerList.innerHTML = `
                <div class="peer-list-loading">
                    <div class="spinner"></div>
                    <p>Discovering peers...</p>
                </div>
            `;

            try {
                const peers = await this.sdk.discoverPeers();
                this.peers.clear();

                if (peers.length === 0) {
                    peerList.innerHTML = '<p class="empty-state">No peers found</p>';
                    return;
                }

                peerList.innerHTML = '';
                peers.forEach(peer => this.addPeer(peer));
            } catch (error) {
                peerList.innerHTML = `<p class="error-state">Failed to discover peers: ${error.message}</p>`;
            }
        }

        addPeer(peer) {
            this.peers.set(peer.peer_id, peer);

            const peerList = this.container.querySelector('#peer-list');
            const emptyState = peerList.querySelector('.empty-state');
            if (emptyState) {
                emptyState.remove();
            }

            const peerItem = document.createElement('div');
            peerItem.className = 'peer-item';
            peerItem.id = `peer-${peer.peer_id}`;
            peerItem.innerHTML = `
                <div class="peer-icon">
                    ${this._getPeerIcon(peer.device_type)}
                </div>
                <div class="peer-info">
                    <div class="peer-name">${this._escapeHtml(peer.name)}</div>
                    <div class="peer-meta">
                        <span class="peer-type">${peer.device_type}</span>
                        <span class="peer-status-badge status-${peer.status || 'disconnected'}">
                            ${peer.status || 'disconnected'}
                        </span>
                    </div>
                </div>
                <div class="peer-quality">
                    <div class="signal-strength signal-${this._getSignalStrength(peer)}">
                        <span></span><span></span><span></span>
                    </div>
                </div>
            `;

            peerItem.addEventListener('click', () => {
                this.showPeerDetails(peer.peer_id);
            });

            peerList.appendChild(peerItem);
        }

        showPeerDetails(peerId) {
            const peer = this.peers.get(peerId);
            if (!peer) return;

            this.selectedPeer = peerId;
            const detailsPanel = this.container.querySelector('#peer-details');

            this.container.querySelector('#peer-details-name').textContent = peer.name;
            this.container.querySelector('#peer-status').textContent = peer.status || 'disconnected';
            this.container.querySelector('#peer-id').textContent = peer.peer_id;
            this.container.querySelector('#peer-device-type').textContent = peer.device_type;

            // Capabilities
            const capabilitiesList = this.container.querySelector('#capabilities-list');
            if (peer.capabilities && peer.capabilities.length > 0) {
                capabilitiesList.innerHTML = peer.capabilities.map(cap =>
                    `<span class="capability-badge">${cap}</span>`
                ).join('');
            } else {
                capabilitiesList.innerHTML = '<span class="capability-badge">None</span>';
            }

            // Update connection quality
            this.updateConnectionQuality(peer);

            detailsPanel.classList.remove('hidden');

            // Highlight selected peer
            this.container.querySelectorAll('.peer-item').forEach(item => {
                item.classList.remove('selected');
            });
            this.container.querySelector(`#peer-${peerId}`).classList.add('selected');
        }

        hideDetails() {
            this.container.querySelector('#peer-details').classList.add('hidden');
            this.selectedPeer = null;

            this.container.querySelectorAll('.peer-item').forEach(item => {
                item.classList.remove('selected');
            });
        }

        updatePeerStatus(peerId, status) {
            const peer = this.peers.get(peerId);
            if (peer) {
                peer.status = status;

                const peerItem = this.container.querySelector(`#peer-${peerId}`);
                if (peerItem) {
                    const statusBadge = peerItem.querySelector('.peer-status-badge');
                    statusBadge.className = `peer-status-badge status-${status}`;
                    statusBadge.textContent = status;
                }

                if (this.selectedPeer === peerId) {
                    this.container.querySelector('#peer-status').textContent = status;
                }
            }
        }

        updatePeerStats(peerId, stats) {
            const peer = this.peers.get(peerId);
            if (peer) {
                peer.stats = stats;

                if (this.selectedPeer === peerId) {
                    this.container.querySelector('#peer-latency').textContent = `${stats.latency} ms`;
                    this.container.querySelector('#peer-bandwidth').textContent =
                        `${Math.round(stats.bandwidth / 1000)} kbps`;
                    this.updateConnectionQuality(peer);
                }
            }
        }

        updateConnectionQuality(peer) {
            const stats = peer.stats || {};
            const latency = stats.latency || 0;

            let quality = 'excellent';
            let qualityPercent = 100;

            if (latency > 200) {
                quality = 'poor';
                qualityPercent = 30;
            } else if (latency > 100) {
                quality = 'fair';
                qualityPercent = 60;
            } else if (latency > 50) {
                quality = 'good';
                qualityPercent = 80;
            }

            const qualityFill = this.container.querySelector('#quality-fill');
            const qualityText = this.container.querySelector('#quality-text');

            if (qualityFill && qualityText) {
                qualityFill.style.width = `${qualityPercent}%`;
                qualityFill.className = `quality-fill quality-${quality}`;
                qualityText.textContent = quality.charAt(0).toUpperCase() + quality.slice(1);
            }
        }

        async connectToPeer(peerId) {
            try {
                await this.sdk.connectToPeer(peerId);
                this.updatePeerStatus(peerId, 'connecting');
            } catch (error) {
                this.showTroubleshooting(`Failed to connect: ${error.message}`);
            }
        }

        async disconnectFromPeer(peerId) {
            try {
                await this.sdk.disconnectFromPeer(peerId);
                this.updatePeerStatus(peerId, 'disconnected');
            } catch (error) {
                this.showTroubleshooting(`Failed to disconnect: ${error.message}`);
            }
        }

        async testConnection(peerId) {
            const troubleshootingInfo = this.container.querySelector('#troubleshooting-info');
            troubleshootingInfo.innerHTML = '<p>Testing connection...</p>';

            try {
                const result = await this.sdk.testConnection(peerId);
                troubleshootingInfo.innerHTML = `
                    <div class="test-result test-${result.success ? 'success' : 'failure'}">
                        <p><strong>Connection Test ${result.success ? 'Passed' : 'Failed'}</strong></p>
                        <p>Latency: ${result.latency} ms</p>
                        <p>Packet Loss: ${result.packetLoss}%</p>
                        ${result.message ? `<p>${result.message}</p>` : ''}
                    </div>
                `;
            } catch (error) {
                troubleshootingInfo.innerHTML = `
                    <div class="test-result test-failure">
                        <p><strong>Connection Test Failed</strong></p>
                        <p>${error.message}</p>
                    </div>
                `;
            }
        }

        showTroubleshooting(message) {
            const troubleshootingInfo = this.container.querySelector('#troubleshooting-info');
            troubleshootingInfo.innerHTML = `<p class="error-message">${message}</p>`;
        }

        _getPeerIcon(deviceType) {
            const icons = {
                desktop: '<svg viewBox="0 0 24 24" width="24" height="24"><path fill="currentColor" d="M21,16H3V4H21M21,2H3C1.89,2 1,2.89 1,4V16A2,2 0 0,0 3,18H10V20H8V22H16V20H14V18H21A2,2 0 0,0 23,16V4C23,2.89 22.1,2 21,2Z"/></svg>',
                laptop: '<svg viewBox="0 0 24 24" width="24" height="24"><path fill="currentColor" d="M4,6H20V16H4M20,18A2,2 0 0,0 22,16V6C22,4.89 21.1,4 20,4H4C2.89,4 2,4.89 2,6V16A2,2 0 0,0 4,18H0V20H24V18H20Z"/></svg>',
                mobile: '<svg viewBox="0 0 24 24" width="24" height="24"><path fill="currentColor" d="M17,19H7V5H17M17,1H7C5.89,1 5,1.89 5,3V21A2,2 0 0,0 7,23H17A2,2 0 0,0 19,21V3C19,1.89 18.1,1 17,1Z"/></svg>',
                tablet: '<svg viewBox="0 0 24 24" width="24" height="24"><path fill="currentColor" d="M19,18H5V6H19M21,4H3C1.89,4 1,4.89 1,6V18A2,2 0 0,0 3,20H21A2,2 0 0,0 23,18V6C23,4.89 22.1,4 21,4Z"/></svg>'
            };
            return icons[deviceType.toLowerCase()] || icons.desktop;
        }

        _getSignalStrength(peer) {
            const stats = peer.stats || {};
            const latency = stats.latency || 999;

            if (latency < 50) return 'strong';
            if (latency < 100) return 'medium';
            return 'weak';
        }

        _escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }
    }

    // Export UI components
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = {
            FileTransferUI,
            VideoPlayerUI,
            CommandTerminalUI,
            PeerManagementUI
        };
    } else {
        global.KizunaUI = {
            FileTransferUI,
            VideoPlayerUI,
            CommandTerminalUI,
            PeerManagementUI
        };
    }

})(typeof window !== 'undefined' ? window : this);
