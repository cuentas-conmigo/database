resource "aws_dynamodb_table" "pey_events" {
  name         = "${var.name}-pey-events"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "eventId"

  attribute {
    name = "eventId"
    type = "S"
  }

  server_side_encryption {
    enabled = true
  }
}

# Holds the Pey API key. Created empty: the value is added by a person, never by OpenTofu.
resource "aws_secretsmanager_secret" "pey" {
  name                    = "${var.name}/pey"
  description             = "Pey API key for the payments backend."
  recovery_window_in_days = 7
}

resource "aws_ecr_repository" "api" {
  name                 = "${var.name}-api"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

resource "aws_ecr_lifecycle_policy" "api" {
  repository = aws_ecr_repository.api.name
  policy = jsonencode({
    rules = [{
      rulePriority = 1
      description  = "Keep the last 10 images"
      selection = {
        tagStatus   = "any"
        countType   = "imageCountMoreThan"
        countNumber = 10
      }
      action = { type = "expire" }
    }]
  })
}
