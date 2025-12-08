/**
 * Kizuna Mobile Optimization
 * 
 * Mobile-specific touch interfaces, gestures, and optimizations
 * for browser support on mobile devices.
 * 
 * @version 1.0.0
 */

(function (global) {
    'use strict';

    /**
     * Mobile Touch Handler
     * Handles touch events and gestures for mobile interfaces
     */
    class MobileTouchHandler {
        constructor() {
            this.touchStartX = 0;
            this.touchStartY = 0;
            this.touchEndX = 0;
            this.touchEndY = 0;
            this.minSwipeDistance = 50;
            this.tapTimeout = null;
            this.longPressTimeout = null;
            this.longPressDuration = 500;
            this.doubleTapDelay = 300;
            this.lastTapTime = 0;
        }

        /**
         * Initialize touch handling on an element
         */
        init(element, callbacks = {}) {
            this.callbacks = {
                onSwipeLeft: callbacks.onSwipeLeft || (() => { }),
                onSwipeRight: callbacks.onSwipeRight || (() => { }),
                onSwipeUp: callbacks.onSwipeUp || (() => { }),
                onSwipeDown: callbacks.onSwipeDown || (() => { }),
                onTap: callbacks.onTap || (() => { }),
                onDoubleTap: callbacks.onDoubleTap || (() => { }),
                onLongPress: callbacks.onLongPress || (() => { }),
                onPinch: callbacks.onPinch || (() => { }),
            };

            element.addEventListener('touchstart', this.handleTouchStart.bind(this), { passive: false });
            element.addEventListener('touchmove', this.handleTouchMove.bind(this), { passive: false });
            element.addEventListener('touchend', this.handleTouchEnd.bind(this), { passive: false });
        }

        handleTouchStart(e) {
            if (e.touches.length === 1) {
                this.touchStartX = e.touches[0].clientX;
                this.touchStartY = e.touches[0].clientY;

                // Long press detection
                this.longPressTimeout = setTimeout(() => {
                    this.callbacks.onLongPress(e);
                }, this.longPressDuration);
            } else if (e.touches.length === 2) {
                // Pinch gesture detection
                this.initialPinchDistance = this.getPinchDistance(e.touches);
            }
        }

        handleTouchMove(e) {
            // Cancel long press if finger moves
            if (this.longPressTimeout) {
                clearTimeout(this.longPressTimeout);
                this.longPressTimeout = null;
            }

            if (e.touches.length === 2) {
                e.preventDefault();
                const currentDistance = this.getPinchDistance(e.touches);
                const scale = currentDistance / this.initialPinchDistance;
                this.callbacks.onPinch({ scale, distance: currentDistance });
            }
        }

        handleTouchEnd(e) {
            if (this.longPressTimeout) {
                clearTimeout(this.longPressTimeout);
                this.longPressTimeout = null;
            }

            if (e.changedTouches.length === 1) {
                this.touchEndX = e.changedTouches[0].clientX;
                this.touchEndY = e.changedTouches[0].clientY;

                this.handleGesture(e);
            }
        }

        handleGesture(e) {
            const deltaX = this.touchEndX - this.touchStartX;
            const deltaY = this.touchEndY - this.touchStartY;
            const absDeltaX = Math.abs(deltaX);
            const absDeltaY = Math.abs(deltaY);

            // Check for swipe gestures
            if (absDeltaX > this.minSwipeDistance || absDeltaY > this.minSwipeDistance) {
                if (absDeltaX > absDeltaY) {
                    // Horizontal swipe
                    if (deltaX > 0) {
                        this.callbacks.onSwipeRight(e);
                    } else {
                        this.callbacks.onSwipeLeft(e);
                    }
                } else {
                    // Vertical swipe
                    if (deltaY > 0) {
                        this.callbacks.onSwipeDown(e);
                    } else {
                        this.callbacks.onSwipeUp(e);
                    }
                }
            } else {
                // Check for tap or double tap
                const currentTime = Date.now();
                const timeSinceLastTap = currentTime - this.lastTapTime;

                if (timeSinceLastTap < this.doubleTapDelay && timeSinceLastTap > 0) {
                    this.callbacks.onDoubleTap(e);
                    this.lastTapTime = 0;
                } else {
                    this.lastTapTime = currentTime;
                    setTimeout(() => {
                        if (this.lastTapTime === currentTime) {
                            this.callbacks.onTap(e);
                        }
                    }, this.doubleTapDelay);
                }
            }
        }

        getPinchDistance(touches) {
            const dx = touches[0].clientX - touches[1].clientX;
            const dy = touches[0].clientY - touches[1].clientY;
            return Math.sqrt(dx * dx + dy * dy);
        }
    }

    /**
     * Mobile File Transfer UI
     * Touch-optimized file transfer interface for mobile
     */
    class MobileFileTransferUI {
        constructor(container, sdk) {
            this.container = container;
            this.sdk = sdk;
            this.touchHandler = new MobileTouchHandler();
            this.init();
        }

        init() {
            this.render();
            this.setupTouchGestures();
        }

        render() {
            this.container.innerHTML = `
                <div class="mobile-file-transfer">
                    <div class="mobile-header">
                        <h2>File Transfer</h2>
                        <button class="mobile-btn-icon" id="mobile-menu-btn">
                            <svg viewBox="0 0 24 24" width="24" height="24">
                                <path fill="currentColor" d="M3,6H21V8H3V6M3,11H21V13H3V11M3,16H21V18H3V16Z"/>
                            </svg>
                        </button>
                    </div>

                    <div class="mobile-drop-zone" id="mobile-drop-zone">
                        <svg class="mobile-upload-icon" viewBox="0 0 24 24" width="64" height="64">
                            <path fill="currentColor" d="M9,16V10H5L12,3L19,10H15V16H9M5,20V18H19V20H5Z"/>
                        </svg>
                        <p class="mobile-drop-text">Tap to select files</p>
                        <p class="mobile-drop-subtext">or drag and drop</p>
                        <input type="file" id="mobile-file-input" multiple hidden>
                    </div>

                    <div class="mobile-transfer-list" id="mobile-transfer-list">
                        <div class="mobile-section-header">
                            <h3>Active Transfers</h3>
                            <span class="mobile-badge" id="transfer-count">0</span>
                        </div>
                        <div class="mobile-transfers" id="mobile-transfers">
                            <p class="mobile-empty-state">No active transfers</p>
                        </div>
                    </div>
                </div>
            `;
        }

        setupTouchGestures() {
            const dropZone = this.container.querySelector('#mobile-drop-zone');
            const fileInput = this.container.querySelector('#mobile-file-input');

            // Tap to select files
            dropZone.addEventListener('click', () => {
                fileInput.click();
            });

            fileInput.addEventListener('change', (e) => {
                const files = Array.from(e.target.files);
                this.handleFiles(files);
                fileInput.value = '';
            });

            // Swipe gestures on transfer items
            const transferList = this.container.querySelector('#mobile-transfers');
            this.touchHandler.init(transferList, {
                onSwipeLeft: (e) => {
                    const transferItem = e.target.closest('.mobile-transfer-item');
                    if (transferItem) {
                        this.showTransferActions(transferItem);
                    }
                },
                onSwipeRight: (e) => {
                    const transferItem = e.target.closest('.mobile-transfer-item');
                    if (transferItem) {
                        this.hideTransferActions(transferItem);
                    }
                },
                onLongPress: (e) => {
                    const transferItem = e.target.closest('.mobile-transfer-item');
                    if (transferItem) {
                        this.showTransferOptions(transferItem);
                    }
                }
            });
        }

        handleFiles(files) {
            if (files.length === 0) return;

            const transfersContainer = this.container.querySelector('#mobile-transfers');
            const emptyState = transfersContainer.querySelector('.mobile-empty-state');
            if (emptyState) {
                emptyState.remove();
            }

            files.forEach(file => {
                this.addTransferItem(file);
            });

            this.updateTransferCount();
        }

        addTransferItem(file) {
            const transfersContainer = this.container.querySelector('#mobile-transfers');
            const transferId = this.generateId();

            const transferItem = document.createElement('div');
            transferItem.className = 'mobile-transfer-item';
            transferItem.dataset.transferId = transferId;
            transferItem.innerHTML = `
                <div class="mobile-transfer-content">
                    <div class="mobile-transfer-icon">
                        <svg viewBox="0 0 24 24" width="32" height="32">
                            <path fill="currentColor" d="M14,2H6A2,2 0 0,0 4,4V20A2,2 0 0,0 6,22H18A2,2 0 0,0 20,20V8L14,2Z"/>
                        </svg>
                    </div>
                    <div class="mobile-transfer-info">
                        <div class="mobile-transfer-name">${this.escapeHtml(file.name)}</div>
                        <div class="mobile-transfer-meta">
                            <span class="mobile-transfer-size">${this.formatBytes(file.size)}</span>
                            <span class="mobile-transfer-status">Uploading</span>
                        </div>
                        <div class="mobile-progress-bar">
                            <div class="mobile-progress-fill" style="width: 0%"></div>
                        </div>
                    </div>
                </div>
                <div class="mobile-transfer-actions">
                    <button class="mobile-action-btn mobile-cancel-btn">
                        <svg viewBox="0 0 24 24" width="24" height="24">
                            <path fill="currentColor" d="M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z"/>
                        </svg>
                    </button>
                </div>
            `;

            transfersContainer.appendChild(transferItem);

            // Setup cancel button
            const cancelBtn = transferItem.querySelector('.mobile-cancel-btn');
            cancelBtn.addEventListener('click', () => {
                this.cancelTransfer(transferId);
            });

            // Simulate upload progress
            this.simulateUpload(transferId);
        }

        simulateUpload(transferId) {
            let progress = 0;
            const interval = setInterval(() => {
                progress += Math.random() * 15;
                if (progress >= 100) {
                    progress = 100;
                    clearInterval(interval);
                    this.completeTransfer(transferId);
                }
                this.updateTransferProgress(transferId, progress);
            }, 500);
        }

        updateTransferProgress(transferId, progress) {
            const transferItem = this.container.querySelector(`[data-transfer-id="${transferId}"]`);
            if (transferItem) {
                const progressFill = transferItem.querySelector('.mobile-progress-fill');
                progressFill.style.width = `${Math.round(progress)}%`;
            }
        }

        completeTransfer(transferId) {
            const transferItem = this.container.querySelector(`[data-transfer-id="${transferId}"]`);
            if (transferItem) {
                const status = transferItem.querySelector('.mobile-transfer-status');
                status.textContent = 'Complete';
                status.style.color = '#4CAF50';

                setTimeout(() => {
                    transferItem.remove();
                    this.updateTransferCount();
                }, 2000);
            }
        }

        cancelTransfer(transferId) {
            const transferItem = this.container.querySelector(`[data-transfer-id="${transferId}"]`);
            if (transferItem) {
                transferItem.remove();
                this.updateTransferCount();
            }
        }

        showTransferActions(transferItem) {
            transferItem.classList.add('show-actions');
        }

        hideTransferActions(transferItem) {
            transferItem.classList.remove('show-actions');
        }

        showTransferOptions(transferItem) {
            // Show bottom sheet with transfer options
            const transferId = transferItem.dataset.transferId;
            // TODO: Implement bottom sheet modal
            console.log('Show options for transfer:', transferId);
        }

        updateTransferCount() {
            const transfers = this.container.querySelectorAll('.mobile-transfer-item');
            const countBadge = this.container.querySelector('#transfer-count');
            countBadge.textContent = transfers.length;

            if (transfers.length === 0) {
                const transfersContainer = this.container.querySelector('#mobile-transfers');
                transfersContainer.innerHTML = '<p class="mobile-empty-state">No active transfers</p>';
            }
        }

        formatBytes(bytes) {
            if (bytes === 0) return '0 B';
            const k = 1024;
            const sizes = ['B', 'KB', 'MB', 'GB'];
            const i = Math.floor(Math.log(bytes) / Math.log(k));
            return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i];
        }

        escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }

        generateId() {
            return 'transfer-' + Date.now() + '-' + Math.random().toString(36).substr(2, 9);
        }
    }

    /**
     * Mobile Video Player UI
     * Touch-optimized video player for mobile
     */
    class MobileVideoPlayerUI {
        constructor(container, sdk) {
            this.container = container;
            this.sdk = sdk;
            this.touchHandler = new MobileTouchHandler();
            this.controlsVisible = false;
            this.controlsTimeout = null;
            this.init();
        }

        init() {
            this.render();
            this.setupTouchGestures();
        }

        render() {
            this.container.innerHTML = `
                <div class="mobile-video-player">
                    <div class="mobile-video-container" id="mobile-video-container">
                        <video id="mobile-video-element" playsinline webkit-playsinline></video>
                        
                        <div class="mobile-video-overlay" id="mobile-video-overlay">
                            <svg class="mobile-video-icon" viewBox="0 0 24 24" width="64" height="64">
                                <path fill="currentColor" d="M17,10.5V7A1,1 0 0,0 16,6H4A1,1 0 0,0 3,7V17A1,1 0 0,0 4,18H16A1,1 0 0,0 17,17V13.5L21,17.5V6.5L17,10.5Z"/>
                            </svg>
                            <p>No video stream</p>
                            <button class="mobile-btn-primary" id="mobile-start-stream">Start Stream</button>
                        </div>

                        <div class="mobile-video-controls" id="mobile-video-controls">
                            <div class="mobile-controls-top">
                                <button class="mobile-control-btn" id="mobile-back-btn">
                                    <svg viewBox="0 0 24 24" width="24" height="24">
                                        <path fill="currentColor" d="M20,11V13H8L13.5,18.5L12.08,19.92L4.16,12L12.08,4.08L13.5,5.5L8,11H20Z"/>
                                    </svg>
                                </button>
                                <div class="mobile-stream-info">
                                    <span class="mobile-stream-status">Live</span>
                                </div>
                            </div>

                            <div class="mobile-controls-center">
                                <button class="mobile-control-btn-large" id="mobile-play-pause">
                                    <svg class="mobile-play-icon" viewBox="0 0 24 24" width="48" height="48">
                                        <path fill="currentColor" d="M8,5.14V19.14L19,12.14L8,5.14Z"/>
                                    </svg>
                                    <svg class="mobile-pause-icon hidden" viewBox="0 0 24 24" width="48" height="48">
                                        <path fill="currentColor" d="M14,19H18V5H14M6,19H10V5H6V19Z"/>
                                    </svg>
                                </button>
                            </div>

                            <div class="mobile-controls-bottom">
                                <button class="mobile-control-btn" id="mobile-quality-btn">
                                    <svg viewBox="0 0 24 24" width="24" height="24">
                                        <path fill="currentColor" d="M12,2A10,10 0 0,0 2,12A10,10 0 0,0 12,22A10,10 0 0,0 22,12A10,10 0 0,0 12,2M12,4A8,8 0 0,1 20,12A8,8 0 0,1 12,20A8,8 0 0,1 4,12A8,8 0 0,1 12,4Z"/>
                                    </svg>
                                    <span>Auto</span>
                                </button>
                                <button class="mobile-control-btn" id="mobile-fullscreen-btn">
                                    <svg viewBox="0 0 24 24" width="24" height="24">
                                        <path fill="currentColor" d="M5,5H10V7H7V10H5V5M14,5H19V10H17V7H14V5M17,14H19V19H14V17H17V14M10,17V19H5V14H7V17H10Z"/>
                                    </svg>
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            `;
        }

        setupTouchGestures() {
            const videoContainer = this.container.querySelector('#mobile-video-container');
            const videoElement = this.container.querySelector('#mobile-video-element');
            const startStreamBtn = this.container.querySelector('#mobile-start-stream');
            const playPauseBtn = this.container.querySelector('#mobile-play-pause');
            const fullscreenBtn = this.container.querySelector('#mobile-fullscreen-btn');

            // Touch gestures for video control
            this.touchHandler.init(videoContainer, {
                onTap: () => {
                    this.toggleControls();
                },
                onDoubleTap: () => {
                    if (videoElement.paused) {
                        videoElement.play();
                    } else {
                        videoElement.pause();
                    }
                },
                onSwipeUp: () => {
                    this.enterFullscreen();
                },
                onSwipeDown: () => {
                    this.exitFullscreen();
                }
            });

            // Button handlers
            startStreamBtn.addEventListener('click', () => {
                this.startStream();
            });

            playPauseBtn.addEventListener('click', () => {
                if (videoElement.paused) {
                    videoElement.play();
                } else {
                    videoElement.pause();
                }
            });

            fullscreenBtn.addEventListener('click', () => {
                this.toggleFullscreen();
            });

            // Video events
            videoElement.addEventListener('play', () => {
                this.updatePlayPauseButton(false);
            });

            videoElement.addEventListener('pause', () => {
                this.updatePlayPauseButton(true);
            });
        }

        async startStream() {
            try {
                const stream = await this.sdk.requestVideoStream();
                const videoElement = this.container.querySelector('#mobile-video-element');
                videoElement.srcObject = stream;

                this.container.querySelector('#mobile-video-overlay').classList.add('hidden');
                this.showControls();
            } catch (error) {
                console.error('Failed to start stream:', error);
            }
        }

        toggleControls() {
            if (this.controlsVisible) {
                this.hideControls();
            } else {
                this.showControls();
            }
        }

        showControls() {
            const controls = this.container.querySelector('#mobile-video-controls');
            controls.classList.add('visible');
            this.controlsVisible = true;

            // Auto-hide controls after 3 seconds
            if (this.controlsTimeout) {
                clearTimeout(this.controlsTimeout);
            }
            this.controlsTimeout = setTimeout(() => {
                this.hideControls();
            }, 3000);
        }

        hideControls() {
            const controls = this.container.querySelector('#mobile-video-controls');
            controls.classList.remove('visible');
            this.controlsVisible = false;
        }

        updatePlayPauseButton(isPaused) {
            const playIcon = this.container.querySelector('.mobile-play-icon');
            const pauseIcon = this.container.querySelector('.mobile-pause-icon');

            if (isPaused) {
                playIcon.classList.remove('hidden');
                pauseIcon.classList.add('hidden');
            } else {
                playIcon.classList.add('hidden');
                pauseIcon.classList.remove('hidden');
            }
        }

        toggleFullscreen() {
            if (!document.fullscreenElement) {
                this.enterFullscreen();
            } else {
                this.exitFullscreen();
            }
        }

        enterFullscreen() {
            const videoContainer = this.container.querySelector('#mobile-video-container');
            if (videoContainer.requestFullscreen) {
                videoContainer.requestFullscreen();
            } else if (videoContainer.webkitRequestFullscreen) {
                videoContainer.webkitRequestFullscreen();
            } else if (videoContainer.mozRequestFullScreen) {
                videoContainer.mozRequestFullScreen();
            }
        }

        exitFullscreen() {
            if (document.exitFullscreen) {
                document.exitFullscreen();
            } else if (document.webkitExitFullscreen) {
                document.webkitExitFullscreen();
            } else if (document.mozCancelFullScreen) {
                document.mozCancelFullScreen();
            }
        }
    }

    // Export mobile components
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = {
            MobileTouchHandler,
            MobileFileTransferUI,
            MobileVideoPlayerUI
        };
    } else {
        global.KizunaMobile = {
            MobileTouchHandler,
            MobileFileTransferUI,
            MobileVideoPlayerUI
        };
    }

})(typeof window !== 'undefined' ? window : this);
