variable "kubeconfig_path" {
  description = "Absolute path to the kubeconfig used by the Helm provider."
  type        = string
  default     = "~/.kube/config"
}

variable "release_name" {
  description = "Helm release name for the claw deployment."
  type        = string
  default     = "claw"
}

variable "namespace" {
  description = "Kubernetes namespace to install the release into."
  type        = string
  default     = "claw-system"
}

variable "chart_path" {
  description = "Path or chart reference to the claw Helm chart."
  type        = string
  default     = "../helm/claw"
}

variable "values_files" {
  description = "Optional list of Helm values files to pass to the release."
  type        = list(string)
  default     = []
}

variable "image_repository" {
  description = "Container image repository to deploy."
  type        = string
  default     = "ghcr.io/shree-git/claw-vcs"
}

variable "image_tag" {
  description = "Container image tag to deploy."
  type        = string
  default     = "latest"
}
