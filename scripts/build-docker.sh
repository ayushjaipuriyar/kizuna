#!/bin/bash
set -e

# Multi-architecture Docker image build script for Kizuna

IMAGE_NAME=${IMAGE_NAME:-kizuna}
IMAGE_TAG=${IMAGE_TAG:-latest}
PLATFORMS=${PLATFORMS:-"linux/amd64,linux/arm64"}
PUSH=${PUSH:-false}

echo "Building multi-architecture Docker image..."
echo "Image: $IMAGE_NAME:$IMAGE_TAG"
echo "Platforms: $PLATFORMS"

# Create buildx builder if it doesn't exist
if ! docker buildx ls | grep -q kizuna-builder; then
    echo "Creating buildx builder..."
    docker buildx create --use --name kizuna-builder --driver docker-container
else
    echo "Using existing buildx builder..."
    docker buildx use kizuna-builder
fi

# Build arguments
BUILD_ARGS=""
if [ "$PUSH" = "true" ]; then
    BUILD_ARGS="--push"
else
    BUILD_ARGS="--load"
fi

# Build the image
echo "Building image..."
docker buildx build \
    --platform "$PLATFORMS" \
    --tag "$IMAGE_NAME:$IMAGE_TAG" \
    $BUILD_ARGS \
    .

echo "Build complete!"

# Show image info
if [ "$PUSH" = "false" ]; then
    echo ""
    echo "Image information:"
    docker images "$IMAGE_NAME:$IMAGE_TAG"
fi
