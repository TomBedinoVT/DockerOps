use anyhow::Result;
use std::path::Path;
use std::fs;
use std::process::Command;
use git2::Repository;
use serde_yaml::Value;

use crate::database::Database;
use crate::models::{Image, Stack, StackDefinition};

pub struct Commands {
    db: Database,
}

impl Commands {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn watch(&self, github_url: &str) -> Result<()> {
        println!("Watching GitHub repository: {}", github_url);
        
        // Clone the repository
        let repo_path = self.clone_repository(github_url).await?;
        println!("Repository cloned to: {}", repo_path);
        
        // Process stacks and deploy them
        self.process_and_deploy_stacks(&repo_path, github_url, false).await?;
        
        // Clean up cloned repository
        if let Err(e) = fs::remove_dir_all(&repo_path) {
            println!("Warning: Could not clean up repository directory: {}", e);
        }
        
        Ok(())
    }

    pub async fn reconcile(&self) -> Result<()> {
        println!("Reconciling database...");
        
        // Get all stacks and display them
        let stacks = self.db.get_all_stacks().await?;
        println!("Found {} stacks in database:", stacks.len());
        
        for stack in &stacks {
            println!("  - {} (status: {}, hash: {})", stack.name, stack.status, stack.hash);
        }
        
        // Get all images and display them
        let images = self.db.get_all_images().await?;
        println!("\nFound {} images in database:", images.len());
        
        for image in &images {
            println!("  - {} (referenced {} times)", image.name, image.reference_count);
        }
        
        // For reconcile, we need to reprocess all repositories
        // This would require storing repository URLs in the database
        // For now, we'll just show the current state
        
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        println!("Stopping DockerOps and cleaning up all resources...");
        
        // Get all stacks from database
        let stacks = self.db.get_all_stacks().await?;
        println!("Found {} stacks to remove", stacks.len());
        
        // Remove all stacks
        for stack in &stacks {
            println!("Removing stack: {}", stack.name);
            self.stop_stack(&stack.name).await?;
        }
        
        // Get all images from database
        let images = self.db.get_all_images().await?;
        println!("Found {} images to remove", images.len());
        
        // Remove all images
        for image in &images {
            println!("Removing image: {}", image.name);
            self.remove_image(&image.name).await?;
        }
        
        // Clean up database
        println!("Cleaning up database...");
        self.db.delete_all_stacks().await?;
        self.db.reset_image_reference_counts().await?;
        self.db.delete_images_with_zero_count().await?;
        
        println!("All stacks and images have been removed.");
        println!("Database connection will be closed.");
        Ok(())
    }

    async fn clone_repository(&self, github_url: &str) -> Result<String> {
        // Convert GitHub URL to clone URL if needed
        let clone_url = if github_url.starts_with("https://github.com/") {
            github_url.to_string()
        } else if github_url.starts_with("github.com/") {
            format!("https://{}", github_url)
        } else {
            github_url.to_string()
        };
        
        // Create temporary directory for cloning
        let temp_dir = format!("temp_repo_{}", chrono::Utc::now().timestamp());
        let repo_path = Path::new(&temp_dir);
        
        println!("Cloning repository from: {}", clone_url);
        
        // Clone the repository
        let _repo = Repository::clone(&clone_url, repo_path)
            .map_err(|e| anyhow::anyhow!("Failed to clone repository: {}", e))?;
        
        Ok(temp_dir)
    }

    async fn process_and_deploy_stacks(&self, repo_path: &str, repository_url: &str, is_reconcile: bool) -> Result<()> {
        println!("Processing stacks from repository...");
        
        // Reset image reference counts at the beginning
        println!("Resetting image reference counts...");
        self.db.reset_image_reference_counts().await?;
        
        // Look for stacks.yaml file
        let stacks_file_path = Path::new(repo_path).join("stacks.yaml");
        if !stacks_file_path.exists() {
            return Err(anyhow::anyhow!("stacks.yaml not found in repository"));
        }
        
        // Read and parse stacks.yaml
        let stacks_content = fs::read_to_string(&stacks_file_path)?;
        let stacks_definitions: Vec<StackDefinition> = serde_yaml::from_str(&stacks_content)?;
        
        println!("Found {} stack definitions:", stacks_definitions.len());
        
        for stack_def in &stacks_definitions {
            println!("Processing stack: {}", stack_def.name);
            
            // Look for the stack directory
            let stack_dir = Path::new(repo_path).join(&stack_def.name);
            if !stack_dir.exists() || !stack_dir.is_dir() {
                println!("  Warning: Stack directory '{}' not found", stack_def.name);
                continue;
            }
            
            // Look for docker-compose file in the stack directory
            let compose_files = vec![
                stack_dir.join("docker-compose.yml"),
                stack_dir.join("docker-compose.yaml"),
                stack_dir.join("compose.yml"),
                stack_dir.join("compose.yaml"),
            ];
            
            let mut compose_file_path = None;
            for compose_file in &compose_files {
                if compose_file.exists() {
                    compose_file_path = Some(compose_file.clone());
                    break;
                }
            }
            
            if compose_file_path.is_none() {
                println!("  Warning: No docker-compose file found in stack directory '{}'", stack_def.name);
                continue;
            }
            
            let compose_path = compose_file_path.unwrap();
            let compose_content = fs::read_to_string(&compose_path)?;
            let compose_hash = self.calculate_md5(&compose_content);
            
            // Calculate relative path for database
            let relative_compose_path = compose_path.strip_prefix(repo_path)
                .unwrap_or(&compose_path)
                .to_string_lossy()
                .replace('\\', "/")
                .to_string();
            
            // Check if stack exists in database
            if let Some(existing_stack) = self.db.get_stack_by_name(&stack_def.name, repository_url).await? {
                if existing_stack.hash != compose_hash {
                    println!("  Stack '{}' has changed (hash: {} -> {})", 
                        stack_def.name, existing_stack.hash, compose_hash);
                    
                    if is_reconcile {
                        // For reconcile, stop the existing stack first
                        println!("  Stopping existing stack '{}'", stack_def.name);
                        self.stop_stack(&stack_def.name).await?;
                    }
                    
                    // Update stack in database
                    self.db.update_stack_hash(&stack_def.name, repository_url, &compose_hash).await?;
                    
                    // Deploy the updated stack
                    println!("  Deploying updated stack '{}'", stack_def.name);
                    self.deploy_stack(&stack_def.name, &compose_path).await?;
                    self.db.update_stack_status(&stack_def.name, repository_url, "deployed").await?;
                } else {
                    println!("  Stack '{}' unchanged", stack_def.name);
                }
            } else {
                // New stack
                println!("  New stack '{}' found, deploying", stack_def.name);
                let stack = Stack::new(
                    stack_def.name.clone(),
                    repository_url.to_string(),
                    relative_compose_path.clone(),
                    compose_hash.clone(),
                );
                self.db.create_stack(&stack).await?;
                
                // Deploy the new stack
                self.deploy_stack(&stack_def.name, &compose_path).await?;
                self.db.update_stack_status(&stack_def.name, repository_url, "deployed").await?;
            }
            
            // Process compose file for image extraction
            self.process_yaml_file(&compose_content, &relative_compose_path).await?;
        }
        
        // Process images: check SHA, pull if needed, remove unused
        println!("Processing images...");
        self.process_images().await?;
        
        Ok(())
    }

    fn calculate_md5(&self, content: &str) -> String {
        let result = md5::compute(content.as_bytes());
        format!("{:x}", result)
    }

    async fn process_yaml_file(&self, content: &str, file_path: &str) -> Result<()> {
        // Parse YAML content
        let yaml_value: Value = match serde_yaml::from_str(content) {
            Ok(value) => value,
            Err(e) => {
                println!("  Warning: Could not parse YAML file {}: {}", file_path, e);
                return Ok(());
            }
        };
        
        // Extract images from YAML structure
        let mut images_found = Vec::new();
        self.extract_images_from_yaml(&yaml_value, &mut images_found);
        
        // Update database with found images
        for image_name in &images_found {
            self.update_image_reference(image_name).await?;
        }
        
        if !images_found.is_empty() {
            println!("  Found {} images in {}: {:?}", images_found.len(), file_path, images_found);
        }
        
        Ok(())
    }

    fn extract_images_from_yaml(&self, value: &Value, images: &mut Vec<String>) {
        match value {
            Value::Mapping(mapping) => {
                for (key, val) in mapping {
                    if let Some(key_str) = key.as_str() {
                        if key_str == "image" {
                            if let Some(image_name) = val.as_str() {
                                if !image_name.is_empty() {
                                    images.push(image_name.to_string());
                                }
                            }
                        } else {
                            // Recursively search in nested structures
                            self.extract_images_from_yaml(val, images);
                        }
                    } else {
                        // Recursively search in nested structures
                        self.extract_images_from_yaml(val, images);
                    }
                }
            }
            Value::Sequence(sequence) => {
                for item in sequence {
                    self.extract_images_from_yaml(item, images);
                }
            }
            _ => {
                // For other types (String, Number, etc.), do nothing
            }
        }
    }

    async fn update_image_reference(&self, image_name: &str) -> Result<()> {
        // Try to get existing image
        if let Some(existing_image) = self.db.get_image_by_name(image_name).await? {
            // Increment reference count
            let new_count = existing_image.reference_count + 1;
            self.db.update_image_reference_count(image_name, new_count).await?;
            println!("    Incremented reference count for '{}' to {}", image_name, new_count);
        } else {
            // Create new image with reference count 1
            let new_image = Image::new(image_name.to_string(), 1);
            self.db.create_image(&new_image).await?;
            println!("    Added new image '{}' with reference count 1", image_name);
        }
        
        Ok(())
    }

    async fn deploy_stack(&self, stack_name: &str, compose_path: &Path) -> Result<()> {
        println!("    Deploying stack '{}' with docker stack deploy", stack_name);
        
        let output = Command::new("docker")
            .args(&["stack", "deploy", "-c", compose_path.to_str().unwrap(), stack_name])
            .output()?;
        
        if output.status.success() {
            println!("    Successfully deployed stack '{}'", stack_name);
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            println!("    Error deploying stack '{}': {}", stack_name, error);
            return Err(anyhow::anyhow!("Failed to deploy stack: {}", error));
        }
        
        Ok(())
    }

    async fn stop_stack(&self, stack_name: &str) -> Result<()> {
        println!("    Stopping stack '{}' with docker stack rm", stack_name);
        
        let output = Command::new("docker")
            .args(&["stack", "rm", stack_name])
            .output()?;
        
        if output.status.success() {
            println!("    Successfully stopped stack '{}'", stack_name);
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            println!("    Warning: Error stopping stack '{}': {}", stack_name, error);
            // Don't return error here as the stack might not exist
        }
        
        Ok(())
    }

    async fn process_images(&self) -> Result<()> {
        // Get all images from database
        let images = self.db.get_all_images().await?;
        println!("  Found {} images in database", images.len());
        
        for image in &images {
            if image.reference_count == 0 {
                // Remove unused images
                println!("  Removing unused image: {}", image.name);
                self.remove_image(&image.name).await?;
            } else {
                // Check and update image if needed
                println!("  Processing image: {} (referenced {} times)", image.name, image.reference_count);
                self.check_and_update_image(&image.name).await?;
            }
        }
        
        // Remove images with zero count from database
        self.db.delete_images_with_zero_count().await?;
        
        Ok(())
    }

    async fn check_and_update_image(&self, image_name: &str) -> Result<()> {
        // Parse image name to get registry, repository, and tag
        let (registry, repository, tag) = self.parse_image_name(image_name);
        
        // Check if image exists locally
        let local_sha = self.get_local_image_sha(image_name).await?;
        
        // Get remote SHA from registry
        let remote_sha = self.get_remote_image_sha(&registry, &repository, &tag).await?;
        
        if let (Some(local), Some(remote)) = (&local_sha, &remote_sha) {
            if local != remote {
                println!("    SHA mismatch for {}: local={}, remote={}", image_name, local, remote);
                println!("    Removing old image and pulling new version");
                self.remove_image(image_name).await?;
                self.pull_image(image_name).await?;
            } else {
                println!("    Image {} is up to date", image_name);
            }
        } else if local_sha.is_none() {
            // Image doesn't exist locally, pull it
            println!("    Image {} not found locally, pulling", image_name);
            self.pull_image(image_name).await?;
        } else {
            println!("    Could not get remote SHA for {}", image_name);
        }
        
        Ok(())
    }

    fn parse_image_name(&self, image_name: &str) -> (String, String, String) {
        // Default to Docker Hub
        let mut registry = "registry-1.docker.io".to_string();
        let mut repository = image_name.to_string();
        let mut tag = "latest".to_string();
        
        // Extract tag first
        if image_name.contains(':') {
            let parts: Vec<&str> = image_name.split(':').collect();
            if parts.len() == 2 {
                repository = parts[0].to_string();
                tag = parts[1].to_string();
            }
        }
        
        // Check if it's a custom registry
        if repository.contains('/') {
            let parts: Vec<&str> = repository.split('/').collect();
            if parts.len() >= 2 {
                if parts[0].contains('.') || parts[0] == "localhost" {
                    // Custom registry
                    registry = parts[0].to_string();
                    repository = parts[1..].join("/");
                }
                // For Docker Hub with organization, keep as is
            }
        }
        
        // For Docker Hub, add library prefix if no organization
        if registry == "registry-1.docker.io" && !repository.contains('/') {
            repository = format!("library/{}", repository);
        }
        
        (registry, repository, tag)
    }

    async fn get_local_image_sha(&self, image_name: &str) -> Result<Option<String>> {
        let output = Command::new("docker")
            .args(&["image", "inspect", image_name, "--format", "{{.Id}}"])
            .output()?;
        
        if output.status.success() {
            let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !sha.is_empty() {
                Ok(Some(sha))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn get_remote_image_sha(&self, registry: &str, repository: &str, tag: &str) -> Result<Option<String>> {
        let url = format!("https://{}/v2/{}/manifests/{}", registry, repository, tag);
        
        let client = reqwest::Client::new();
        let response = client
            .head(&url)
            .header("Accept", "application/vnd.docker.distribution.manifest.v2+json")
            .send()
            .await?;
        
        if response.status().is_success() {
            if let Some(digest) = response.headers().get("Docker-Content-Digest") {
                let sha = digest.to_str()?.to_string();
                Ok(Some(sha))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn remove_image(&self, image_name: &str) -> Result<()> {
        println!("    Removing image: {}", image_name);
        
        let output = Command::new("docker")
            .args(&["image", "rm", image_name])
            .output()?;
        
        if output.status.success() {
            println!("    Successfully removed image: {}", image_name);
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            println!("    Warning: Error removing image {}: {}", image_name, error);
        }
        
        Ok(())
    }

    async fn pull_image(&self, image_name: &str) -> Result<()> {
        println!("    Pulling image: {}", image_name);
        
        let output = Command::new("docker")
            .args(&["image", "pull", image_name])
            .output()?;
        
        if output.status.success() {
            println!("    Successfully pulled image: {}", image_name);
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            println!("    Error pulling image {}: {}", image_name, error);
            return Err(anyhow::anyhow!("Failed to pull image: {}", error));
        }
        
        Ok(())
    }
} 