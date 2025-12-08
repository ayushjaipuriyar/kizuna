/**
 * Kizuna File Transfer API
 * 
 * File transfer functionality for browser clients.
 * Supports upload, download, drag-and-drop, chunking, and resume capability.
 */

(function (global) {
    'use strict';

    /**
     * File Transfer Manager
     */
    class FileTransferManager {
        constructor(sdk) {
            this.sdk = sdk;
            this.activeTransfers = new Map();
            this.transferHistory = [];
            this.chunkSize = 64 * 1024; // 64KB chunks

            // Listen for file transfer messages
            this.sdk.on('message:FileTransferRequest', (msg) => this._handleFileTransferMessage(msg));
            this.sdk.on('message:FileTransferProgress', (msg) => this._handleProgressMessage(msg));
            this.sdk.on('message:FileTransferComplete', (msg) => this._handleCompleteMessage(msg));
            this.sdk.on('message:FileTransferError', (msg) => this._handleErrorMessage(msg));
        }

        /**
         * Upload a file to a peer
         * @param {File} file - File object to upload
         * @param {string} peerId - Target peer ID
         * @param {Object} options - Upload options
         * @returns {Promise<FileTransfer>} Transfer object
         */
        async uploadFile(file, peerId, options = {}) {
            const transferId = this.sdk._generateUUID();

            const transfer = {
                id: transferId,
                type: 'upload',
                file: file,
                fileName: file.name,
                fileSize: file.size,
                mimeType: file.type,
                peerId: peerId,
                status: 'pending',
                progress: 0,
                bytesTransferred: 0,
                startTime: Date.now(),
                endTime: null,
                error: null,
                chunks: [],
                currentChunk: 0,
                totalChunks: Math.ceil(file.size / this.chunkSize),
                resumable: options.resumable !== false,
                onProgress: options.onProgress || null,
                onComplete: options.onComplete || null,
                onError: options.onError || null
            };

            this.activeTransfers.set(transferId, transfer);
            this.sdk._emit('fileTransferStarted', { transfer });

            try {
                // Send file transfer request
                await this.sdk.sendMessage({
                    message_type: 'FileTransferRequest',
                    payload: {
                        transfer_id: transferId,
                        action: 'upload',
                        file_name: file.name,
                        file_size: file.size,
                        mime_type: file.type,
                        chunk_size: this.chunkSize,
                        total_chunks: transfer.totalChunks,
                        peer_id: peerId
                    }
                }, 'file_transfer');

                transfer.status = 'transferring';

                // Start chunked upload
                await this._uploadChunks(transfer);

                return transfer;
            } catch (error) {
                transfer.status = 'failed';
                transfer.error = error.message;
                this.sdk._emit('fileTransferError', { transfer, error });
                if (transfer.onError) transfer.onError(error);
                throw error;
            }
        }

        /**
         * Upload file chunks
         * @private
         */
        async _uploadChunks(transfer) {
            const file = transfer.file;

            for (let i = transfer.currentChunk; i < transfer.totalChunks; i++) {
                if (transfer.status === 'paused') {
                    this.sdk._log('Transfer paused:', transfer.id);
                    return;
                }

                if (transfer.status === 'cancelled') {
                    throw new Error('Transfer cancelled');
                }

                const start = i * this.chunkSize;
                const end = Math.min(start + this.chunkSize, file.size);
                const chunk = file.slice(start, end);

                // Read chunk as base64
                const chunkData = await this._readChunkAsBase64(chunk);

                // Calculate checksum
                const checksum = await this._calculateChecksum(chunkData);

                // Send chunk
                await this.sdk.sendMessage({
                    message_type: 'FileTransferRequest',
                    payload: {
                        transfer_id: transfer.id,
                        action: 'chunk',
                        chunk_id: i,
                        data: chunkData,
                        checksum: checksum
                    }
                }, 'file_transfer');

                transfer.currentChunk = i + 1;
                transfer.bytesTransferred = end;
                transfer.progress = (transfer.bytesTransferred / transfer.fileSize) * 100;

                this.sdk._emit('fileTransferProgress', { transfer });
                if (transfer.onProgress) {
                    transfer.onProgress({
                        progress: transfer.progress,
                        bytesTransferred: transfer.bytesTransferred,
                        totalBytes: transfer.fileSize
                    });
                }
            }

            // Send completion message
            await this.sdk.sendMessage({
                message_type: 'FileTransferRequest',
                payload: {
                    transfer_id: transfer.id,
                    action: 'complete'
                }
            }, 'file_transfer');

            transfer.status = 'completed';
            transfer.endTime = Date.now();
            transfer.progress = 100;

            this.activeTransfers.delete(transfer.id);
            this.transferHistory.push(transfer);

            this.sdk._emit('fileTransferComplete', { transfer });
            if (transfer.onComplete) transfer.onComplete(transfer);
        }

        /**
         * Read chunk as base64
         * @private
         */
        _readChunkAsBase64(chunk) {
            return new Promise((resolve, reject) => {
                const reader = new FileReader();
                reader.onload = () => {
                    const base64 = reader.result.split(',')[1];
                    resolve(base64);
                };
                reader.onerror = reject;
                reader.readAsDataURL(chunk);
            });
        }

        /**
         * Calculate simple checksum
         * @private
         */
        async _calculateChecksum(data) {
            // Simple checksum for now - could use crypto.subtle for SHA-256
            let sum = 0;
            for (let i = 0; i < data.length; i++) {
                sum += data.charCodeAt(i);
            }
            return sum.toString(16);
        }

        /**
         * Download a file from a peer
         * @param {string} fileName - File name to download
         * @param {string} peerId - Source peer ID
         * @param {Object} options - Download options
         * @returns {Promise<FileTransfer>} Transfer object
         */
        async downloadFile(fileName, peerId, options = {}) {
            const transferId = this.sdk._generateUUID();

            const transfer = {
                id: transferId,
                type: 'download',
                fileName: fileName,
                fileSize: 0,
                mimeType: '',
                peerId: peerId,
                status: 'pending',
                progress: 0,
                bytesTransferred: 0,
                startTime: Date.now(),
                endTime: null,
                error: null,
                chunks: [],
                currentChunk: 0,
                totalChunks: 0,
                receivedChunks: new Map(),
                onProgress: options.onProgress || null,
                onComplete: options.onComplete || null,
                onError: options.onError || null
            };

            this.activeTransfers.set(transferId, transfer);
            this.sdk._emit('fileTransferStarted', { transfer });

            try {
                // Send download request
                await this.sdk.sendMessage({
                    message_type: 'FileTransferRequest',
                    payload: {
                        transfer_id: transferId,
                        action: 'download',
                        file_name: fileName,
                        peer_id: peerId
                    }
                }, 'file_transfer');

                transfer.status = 'transferring';

                return transfer;
            } catch (error) {
                transfer.status = 'failed';
                transfer.error = error.message;
                this.sdk._emit('fileTransferError', { transfer, error });
                if (transfer.onError) transfer.onError(error);
                throw error;
            }
        }

        /**
         * Handle file transfer messages
         * @private
         */
        _handleFileTransferMessage(message) {
            const payload = message.payload;
            const transfer = this.activeTransfers.get(payload.transfer_id);

            if (!transfer) {
                this.sdk._log('Transfer not found:', payload.transfer_id);
                return;
            }

            switch (payload.action) {
                case 'accepted':
                    transfer.status = 'transferring';
                    if (payload.file_size) transfer.fileSize = payload.file_size;
                    if (payload.mime_type) transfer.mimeType = payload.mime_type;
                    if (payload.total_chunks) transfer.totalChunks = payload.total_chunks;
                    break;

                case 'chunk':
                    this._handleReceivedChunk(transfer, payload);
                    break;

                case 'rejected':
                    transfer.status = 'failed';
                    transfer.error = payload.reason || 'Transfer rejected';
                    this.sdk._emit('fileTransferError', { transfer, error: new Error(transfer.error) });
                    if (transfer.onError) transfer.onError(new Error(transfer.error));
                    break;
            }
        }

        /**
         * Handle received chunk
         * @private
         */
        _handleReceivedChunk(transfer, payload) {
            transfer.receivedChunks.set(payload.chunk_id, {
                data: payload.data,
                checksum: payload.checksum
            });

            transfer.currentChunk = payload.chunk_id + 1;
            transfer.bytesTransferred = transfer.currentChunk * this.chunkSize;
            transfer.progress = (transfer.bytesTransferred / transfer.fileSize) * 100;

            this.sdk._emit('fileTransferProgress', { transfer });
            if (transfer.onProgress) {
                transfer.onProgress({
                    progress: transfer.progress,
                    bytesTransferred: transfer.bytesTransferred,
                    totalBytes: transfer.fileSize
                });
            }

            // Check if all chunks received
            if (transfer.receivedChunks.size === transfer.totalChunks) {
                this._assembleDownloadedFile(transfer);
            }
        }

        /**
         * Assemble downloaded file from chunks
         * @private
         */
        async _assembleDownloadedFile(transfer) {
            try {
                const chunks = [];
                for (let i = 0; i < transfer.totalChunks; i++) {
                    const chunk = transfer.receivedChunks.get(i);
                    if (!chunk) {
                        throw new Error(`Missing chunk ${i}`);
                    }
                    // Convert base64 to binary
                    const binary = atob(chunk.data);
                    const bytes = new Uint8Array(binary.length);
                    for (let j = 0; j < binary.length; j++) {
                        bytes[j] = binary.charCodeAt(j);
                    }
                    chunks.push(bytes);
                }

                const blob = new Blob(chunks, { type: transfer.mimeType });
                transfer.blob = blob;
                transfer.status = 'completed';
                transfer.endTime = Date.now();
                transfer.progress = 100;

                this.activeTransfers.delete(transfer.id);
                this.transferHistory.push(transfer);

                this.sdk._emit('fileTransferComplete', { transfer });
                if (transfer.onComplete) transfer.onComplete(transfer);

                // Auto-download
                this._triggerDownload(blob, transfer.fileName);
            } catch (error) {
                transfer.status = 'failed';
                transfer.error = error.message;
                this.sdk._emit('fileTransferError', { transfer, error });
                if (transfer.onError) transfer.onError(error);
            }
        }

        /**
         * Trigger browser download
         * @private
         */
        _triggerDownload(blob, fileName) {
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = fileName;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(url);
        }

        /**
         * Handle progress messages
         * @private
         */
        _handleProgressMessage(message) {
            const payload = message.payload;
            const transfer = this.activeTransfers.get(payload.transfer_id);

            if (transfer) {
                transfer.progress = payload.progress;
                transfer.bytesTransferred = payload.bytes_transferred;
                this.sdk._emit('fileTransferProgress', { transfer });
                if (transfer.onProgress) {
                    transfer.onProgress({
                        progress: transfer.progress,
                        bytesTransferred: transfer.bytesTransferred,
                        totalBytes: transfer.fileSize
                    });
                }
            }
        }

        /**
         * Handle complete messages
         * @private
         */
        _handleCompleteMessage(message) {
            const payload = message.payload;
            const transfer = this.activeTransfers.get(payload.transfer_id);

            if (transfer) {
                transfer.status = 'completed';
                transfer.endTime = Date.now();
                transfer.progress = 100;

                this.activeTransfers.delete(transfer.id);
                this.transferHistory.push(transfer);

                this.sdk._emit('fileTransferComplete', { transfer });
                if (transfer.onComplete) transfer.onComplete(transfer);
            }
        }

        /**
         * Handle error messages
         * @private
         */
        _handleErrorMessage(message) {
            const payload = message.payload;
            const transfer = this.activeTransfers.get(payload.transfer_id);

            if (transfer) {
                transfer.status = 'failed';
                transfer.error = payload.error;
                this.sdk._emit('fileTransferError', { transfer, error: new Error(payload.error) });
                if (transfer.onError) transfer.onError(new Error(payload.error));
            }
        }

        /**
         * Pause a transfer
         * @param {string} transferId - Transfer ID
         */
        pauseTransfer(transferId) {
            const transfer = this.activeTransfers.get(transferId);
            if (transfer && transfer.status === 'transferring') {
                transfer.status = 'paused';
                this.sdk._emit('fileTransferPaused', { transfer });
            }
        }

        /**
         * Resume a transfer
         * @param {string} transferId - Transfer ID
         */
        async resumeTransfer(transferId) {
            const transfer = this.activeTransfers.get(transferId);
            if (transfer && transfer.status === 'paused') {
                transfer.status = 'transferring';
                this.sdk._emit('fileTransferResumed', { transfer });

                if (transfer.type === 'upload') {
                    await this._uploadChunks(transfer);
                }
            }
        }

        /**
         * Cancel a transfer
         * @param {string} transferId - Transfer ID
         */
        async cancelTransfer(transferId) {
            const transfer = this.activeTransfers.get(transferId);
            if (transfer) {
                transfer.status = 'cancelled';

                // Send cancellation message
                await this.sdk.sendMessage({
                    message_type: 'FileTransferRequest',
                    payload: {
                        transfer_id: transferId,
                        action: 'cancel'
                    }
                }, 'file_transfer');

                this.activeTransfers.delete(transferId);
                this.sdk._emit('fileTransferCancelled', { transfer });
            }
        }

        /**
         * Get active transfers
         * @returns {Array} Active transfers
         */
        getActiveTransfers() {
            return Array.from(this.activeTransfers.values());
        }

        /**
         * Get transfer history
         * @returns {Array} Transfer history
         */
        getTransferHistory() {
            return this.transferHistory;
        }

        /**
         * Get transfer by ID
         * @param {string} transferId - Transfer ID
         * @returns {Object|null} Transfer object
         */
        getTransfer(transferId) {
            return this.activeTransfers.get(transferId) ||
                this.transferHistory.find(t => t.id === transferId) ||
                null;
        }

        /**
         * Setup drag and drop for an element
         * @param {HTMLElement} element - Element to enable drag and drop
         * @param {Object} options - Options
         */
        setupDragAndDrop(element, options = {}) {
            const peerId = options.peerId;
            const onFilesSelected = options.onFilesSelected || null;

            element.addEventListener('dragover', (e) => {
                e.preventDefault();
                e.stopPropagation();
                element.classList.add('drag-over');
            });

            element.addEventListener('dragleave', (e) => {
                e.preventDefault();
                e.stopPropagation();
                element.classList.remove('drag-over');
            });

            element.addEventListener('drop', async (e) => {
                e.preventDefault();
                e.stopPropagation();
                element.classList.remove('drag-over');

                const files = Array.from(e.dataTransfer.files);

                if (onFilesSelected) {
                    onFilesSelected(files);
                }

                if (peerId) {
                    for (const file of files) {
                        try {
                            await this.uploadFile(file, peerId, options);
                        } catch (error) {
                            this.sdk._error('Upload failed:', error);
                        }
                    }
                }
            });

            this.sdk._log('Drag and drop enabled for element');
        }
    }

    // Export
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = FileTransferManager;
    } else {
        global.KizunaFileTransfer = FileTransferManager;
    }

})(typeof window !== 'undefined' ? window : this);
