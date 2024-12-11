#!/bin/bash

echo "Iniciando criação de secrets no AWS Secrets Manager com arquivo JSON..."

# Criar um secret a partir de um arquivo JSON
aws --endpoint-url=http://localhost:4566 secretsmanager create-secret \
  --name "localstack" \
  --region us-east-1 \
  --description "Este secret foi criado a partir de um arquivo JSON" \
  --secret-string file:///etc/secrets-manager.json

echo "Secrets criados com sucesso a partir de arquivos JSON!"