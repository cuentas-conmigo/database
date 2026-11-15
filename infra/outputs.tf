output "api_url" {
  description = "Public HTTPS base URL; the app uses it as EXPO_PUBLIC_API_URL."
  value       = "https://${aws_cloudfront_distribution.api.domain_name}"
}

output "webhook_url" {
  description = "URL to register in Pey for payment events."
  value       = "https://${aws_cloudfront_distribution.api.domain_name}/v1/webhooks/pey"
}

output "ecr_repository_url" {
  value = aws_ecr_repository.api.repository_url
}

output "instance_id" {
  value = aws_instance.api.id
}

output "payments_table" {
  value = aws_dynamodb_table.payments.name
}

output "pey_events_table" {
  value = aws_dynamodb_table.pey_events.name
}

output "pey_secret_id" {
  value = aws_secretsmanager_secret.pey.name
}
