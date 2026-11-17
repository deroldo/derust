# derust - aws
## Localstack

In `local` and `test` environments `load_aws_config` targets localstack (`http://localhost:4566`, region
`us-east-1`, credentials `test`/`test`). To use another address, set `LOCALSTACK_ENDPOINT` (or
`AWS_ENDPOINT_URL` as a fallback), e.g. `LOCALSTACK_ENDPOINT=http://192.168.1.10:4566`.
