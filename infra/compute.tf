resource "aws_security_group" "api" {
  name        = "${var.name}-api"
  description = "HTTP from CloudFront only; no SSH (use SSM)."
  vpc_id      = data.aws_vpc.default.id

  ingress {
    description     = "HTTP from CloudFront origin-facing servers"
    from_port       = 80
    to_port         = 80
    protocol        = "tcp"
    prefix_list_ids = [data.aws_ec2_managed_prefix_list.cloudfront.id]
  }

  egress {
    description      = "All outbound (ECR, DynamoDB, Secrets Manager, Pey)"
    from_port        = 0
    to_port          = 0
    protocol         = "-1"
    cidr_blocks      = ["0.0.0.0/0"]
    ipv6_cidr_blocks = ["::/0"]
  }
}

locals {
  registry = "${data.aws_caller_identity.current.account_id}.dkr.ecr.${var.region}.amazonaws.com"
}

resource "aws_instance" "api" {
  ami                         = nonsensitive(data.aws_ssm_parameter.al2023_arm64.value)
  instance_type               = var.instance_type
  subnet_id                   = sort(data.aws_subnets.default.ids)[0]
  vpc_security_group_ids      = [aws_security_group.api.id]
  iam_instance_profile        = aws_iam_instance_profile.api.name
  associate_public_ip_address = true

  metadata_options {
    http_endpoint = "enabled"
    http_tokens   = "required"
    # 2 hops so the API container can reach the instance role credentials.
    http_put_response_hop_limit = 2
  }

  root_block_device {
    volume_type           = "gp3"
    volume_size           = 30
    encrypted             = true
    delete_on_termination = true
  }

  user_data = templatefile("${path.module}/user-data.sh.tftpl", {
    region           = var.region
    registry         = local.registry
    image            = "${aws_ecr_repository.api.repository_url}:latest"
    pey_events_table = aws_dynamodb_table.pey_events.name
    pey_secret_id    = aws_secretsmanager_secret.pey.name
  })
  user_data_replace_on_change = false

  tags = { Name = "${var.name}-api" }

  lifecycle {
    ignore_changes = [ami, user_data]
  }
}
