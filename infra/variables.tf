variable "profile" {
  description = "AWS CLI profile of the hackathon account."
  type        = string
  default     = "personal"
}

variable "region" {
  type    = string
  default = "mx-central-1"
}

variable "name" {
  description = "Prefix for every resource name."
  type        = string
  default     = "pagacontufoto"
}

variable "instance_type" {
  type    = string
  default = "t4g.small"
}

variable "budget_email" {
  description = "Address that receives the monthly budget alerts."
  type        = string
  default     = "aguilablanca325@gmail.com"
}

variable "budget_usd" {
  type    = number
  default = 30
}
