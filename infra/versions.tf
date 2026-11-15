terraform {
  required_version = ">= 1.10.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.0"
    }
  }

  backend "s3" {
    bucket       = "pagacontufoto-tfstate-520847769440"
    key          = "pagacontufoto/main.tfstate"
    region       = "mx-central-1"
    profile      = "personal"
    encrypt      = true
    use_lockfile = true
  }
}

provider "aws" {
  profile = var.profile
  region  = var.region
  default_tags {
    tags = { Project = "pagacontufoto", ManagedBy = "opentofu" }
  }
}
