#!/bin/bash
set -e

# Kubernetes deployment script for Kizuna

NAMESPACE=${NAMESPACE:-default}
KUBECTL=${KUBECTL:-kubectl}

echo "Deploying Kizuna to Kubernetes..."
echo "Namespace: $NAMESPACE"

# Create namespace if it doesn't exist
if ! $KUBECTL get namespace "$NAMESPACE" &> /dev/null; then
    echo "Creating namespace $NAMESPACE..."
    $KUBECTL create namespace "$NAMESPACE"
fi

# Apply ConfigMap
echo "Applying ConfigMap..."
$KUBECTL apply -f k8s/configmap.yaml -n "$NAMESPACE"

# Apply Deployment
echo "Applying Deployment..."
$KUBECTL apply -f k8s/deployment.yaml -n "$NAMESPACE"

# Apply Service
echo "Applying Service..."
$KUBECTL apply -f k8s/service.yaml -n "$NAMESPACE"

# Apply HPA
echo "Applying HorizontalPodAutoscaler..."
$KUBECTL apply -f k8s/hpa.yaml -n "$NAMESPACE"

# Wait for deployment to be ready
echo "Waiting for deployment to be ready..."
$KUBECTL rollout status deployment/kizuna -n "$NAMESPACE" --timeout=5m

# Show deployment status
echo ""
echo "Deployment complete!"
echo ""
echo "Pods:"
$KUBECTL get pods -n "$NAMESPACE" -l app=kizuna

echo ""
echo "Services:"
$KUBECTL get services -n "$NAMESPACE" -l app=kizuna

echo ""
echo "HPA:"
$KUBECTL get hpa -n "$NAMESPACE" -l app=kizuna
