use anyhow::Result;
use std::path::Path;
use std::fs;
use std::process::Command;
use serde_yaml::Value;

use crate::database::Database;
use crate::models::{Image, Stack, StackDefinition, VolumeDefinition, VolumeType, NfsConfig};

pub struct Commands {
    db: Database,
}

impl Commands {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn watch(&self, github_url: &str) -> Result<()> {
        println!("Watching GitHub repository: {}", github_url);
        
        // Check if repository is already in cache
        if let Some(cached_repo) = self.db.get_repository_from_cache(github_url).await? {
            return Err(anyhow::anyhow!("Repository '{}' is already being watched (last watch: {})", 
                github_url, cached_repo.last_watch));
        }
        
        // Clone the repository
        let repo_path = self.clone_repository(github_url).await?;
        println!("Repository cloned to: {}", repo_path);
        
        // Process stacks and deploy them
        self.process_and_deploy_stacks(&repo_path, github_url, false).await?;
        
        // Add repository to cache
        self.db.add_repository_to_cache(github_url).await?;
        println!("Repository added to cache");
        
        // Clean up cloned repository
        if let Err(e) = fs::remove_dir_all(&repo_path) {
            println!("Warning: Could not clean up repository directory: {}", e);
        }
        
        Ok(())
    }

    pub async fn reconcile(&self) -> Result<()> {
        println!("Reconciling database...");
        
        // Check if there are any repositories in cache
        let repositories = self.db.get_all_repositories().await?;
        if repositories.is_empty() {
            return Err(anyhow::anyhow!("No repositories found in cache. Please run 'watch' command first."));
        }
        
        println!("Found {} repositories in cache:", repositories.len());
        for repo in &repositories {
            println!("  - {} (last watch: {})", repo.url, repo.last_watch);
        }
        
        // Get all stacks and display them
        let stacks = self.db.get_all_stacks().await?;
        println!("\nFound {} stacks in database:", stacks.len());
        
        for stack in &stacks {
            println!("  - {} (status: {}, hash: {})", stack.name, stack.status, stack.hash);
        }
        
        // Get all images and display them
        let images = self.db.get_all_images().await?;
        println!("\nFound {} images in database:", images.len());
        
        for image in &images {
            println!("  - {} (referenced {} times)", image.name, image.reference_count);
        }
        
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
        self.db.clear_repository_cache().await?;
        
        // Verify cache is cleared
        let repositories = self.db.get_all_repositories().await?;
        if !repositories.is_empty() {
            println!("Warning: Repository cache still contains {} entries, forcing cleanup...", repositories.len());
            self.db.clear_repository_cache().await?;
            
            // Verify again after forced cleanup
            let repositories_after = self.db.get_all_repositories().await?;
            if !repositories_after.is_empty() {
                println!("❌ Cache cleanup failed! Still contains {} entries", repositories_after.len());
                for repo in &repositories_after {
                    println!("  - {}", repo.url);
                }
            } else {
                println!("✅ Cache successfully cleared");
            }
        }
        
        println!("All stacks and images have been removed.");
        println!("Database connection will be closed.");
        Ok(())
    }

    pub fn show_version() {
        println!("DockerOps CLI v{}", env!("CARGO_PKG_VERSION"));
        println!("A Docker Swarm stack manager for GitHub repositories");
        println!("Repository: https://github.com/TomBedinoVT/DockerOps");
    }

    pub async fn debug_cache(&self) -> Result<()> {
        println!("Debug: Checking repository cache...");
        
        let repositories = self.db.get_all_repositories().await?;
        println!("Found {} repositories in cache:", repositories.len());
        
        for repo in &repositories {
            println!("  - {} (last watch: {})", repo.url, repo.last_watch);
        }
        
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
        
        // Create temporary directory for cloning in /tmp
        let temp_dir = format!("/tmp/temp_repo_{}", chrono::Utc::now().timestamp());
        let repo_path = Path::new(&temp_dir);
        
        println!("Cloning repository from: {}", clone_url);
        
        // Check for GitHub token in environment
        let github_token = std::env::var("GITHUB_TOKEN").ok();
        
        // Clone the repository with authentication if token is available
        let mut callbacks = git2::RemoteCallbacks::new();
        
        if let Some(token) = github_token {
            println!("Using GitHub token for authentication");
            // Move token into the closure to ensure it lives long enough
            let token_clone = token.clone();
            callbacks.credentials(move |_url, username_from_url, _allowed_types| {
                git2::Cred::userpass_plaintext(username_from_url.unwrap_or("git"), &token_clone)
            });
        } else {
            println!("No GitHub token found. Trying to clone without authentication...");
            println!("If this fails, set the GITHUB_TOKEN environment variable");
        }
        
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);
        
        let _repo = builder.clone(&clone_url, repo_path)
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
        
        // Process volumes configuration
        let volumes_definitions = self.process_volumes_config(repo_path).await?;
        
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
            let mut compose_content = fs::read_to_string(&compose_path)?;
            
            // Process volumes in compose file if volumes definitions exist
            if let Some(ref volumes_defs) = volumes_definitions {
                println!("  Processing volumes in docker-compose file...");
                compose_content = self.process_compose_volumes(&compose_content, volumes_defs).await?;
                println!("  Volume processing completed");
            }
            
            // Write the modified compose content back to the file
            fs::write(&compose_path, &compose_content)?;
            println!("  Updated docker-compose file with processed volumes at {}", compose_path.to_string_lossy());
            
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
            .args(&["stack", "deploy", "--detach=false", "-c", compose_path.to_str().unwrap(), stack_name])
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

    async fn process_volumes_config(&self, repo_path: &str) -> Result<Option<Vec<VolumeDefinition>>> {
        println!("  Looking for volumes.yaml in: {}", repo_path);
        
        // Look for volumes.yaml file
        let volumes_file_path = Path::new(repo_path).join("volumes.yaml");
        if !volumes_file_path.exists() {
            println!("  No volumes.yaml found at {}, skipping volume processing", volumes_file_path.display());
            return Ok(None);
        }
        
        println!("  Found volumes.yaml at: {}", volumes_file_path.display());
        
        // Read and parse volumes.yaml
        let volumes_content = fs::read_to_string(&volumes_file_path)?;
        println!("  Read volumes.yaml content ({} characters)", volumes_content.len());
        
        let volumes_definitions: Vec<VolumeDefinition> = serde_yaml::from_str(&volumes_content)?;
        println!("  Parsed {} volume definitions from volumes.yaml", volumes_definitions.len());
        
        // Look for nfs.yaml file
        let nfs_file_path = Path::new(repo_path).join("nfs.yaml");
        let nfs_config = if nfs_file_path.exists() {
            println!("  Found nfs.yaml at: {}", nfs_file_path.display());
            let nfs_content = fs::read_to_string(&nfs_file_path)?;
            let config = serde_yaml::from_str::<NfsConfig>(&nfs_content)?;
            println!("  NFS config: {:?}", config);
            Some(config)
        } else {
            println!("  No nfs.yaml found at {}, NFS bindings will be skipped", nfs_file_path.display());
            None
        };
        
        println!("  Processing {} volume definitions", volumes_definitions.len());
        
        let mut volumes_definitions = volumes_definitions;
        
        for volume_def in &mut volumes_definitions {
            println!("  Processing volume definition: {:?}", volume_def);
            
            match volume_def.r#type {
                VolumeType::Volume => {
                    println!("  Processing volume: {} (type: volume, path: {})", 
                        volume_def.id, volume_def.path);
                    // For Docker volumes, we just need to ensure they exist
                    self.ensure_docker_volume_exists(&volume_def.path).await?;
                }
                VolumeType::Binding => {
                    println!("  Processing binding: {} (type: binding, path: {})", 
                        volume_def.id, volume_def.path);
                    if let Some(nfs_config) = &nfs_config {
                        self.process_binding_volume(volume_def, nfs_config, repo_path).await?;
                    } else {
                        println!("    Warning: No NFS configuration found, skipping binding volume");
                    }
                }
            }
        }
        
        println!("  Finished processing all volume definitions");
        Ok(Some(volumes_definitions))
    }

    async fn ensure_docker_volume_exists(&self, volume_name: &str) -> Result<()> {
        let output = Command::new("docker")
            .args(&["volume", "ls", "-q", "-f", &format!("name=^{}$", volume_name)])
            .output()?;
        
        let volume_exists = !String::from_utf8_lossy(&output.stdout).trim().is_empty();
        
        if !volume_exists {
            println!("    Creating Docker volume: {}", volume_name);
            let create_output = Command::new("docker")
                .args(&["volume", "create", volume_name])
                .output()?;
            
            if create_output.status.success() {
                println!("    Successfully created Docker volume: {}", volume_name);
            } else {
                let error = String::from_utf8_lossy(&create_output.stderr);
                return Err(anyhow::anyhow!("Failed to create Docker volume {}: {}", volume_name, error));
            }
        } else {
            println!("    Docker volume already exists: {}", volume_name);
        }
        
        Ok(())
    }

    async fn process_binding_volume(&self, volume_def: &mut VolumeDefinition, nfs_config: &NfsConfig, repo_path: &str) -> Result<()> {
        let local_path = Path::new(repo_path).join(&volume_def.path);
        
        if !local_path.exists() {
            println!("    Warning: Local path does not exist: {}", local_path.display());
            return Ok(());
        }
        
        // Create NFS destination path
        let nfs_dest_path = Path::new(&nfs_config.path).join(&volume_def.id);
        
        println!("    Copying {} to NFS: {}", local_path.display(), nfs_dest_path.display());
        
        // Remove existing file or directory on NFS if it exists
        if nfs_dest_path.exists() {
            let metadata = fs::metadata(&nfs_dest_path)?;
            if metadata.is_dir() {
                println!("    Removing existing directory on NFS: {}", nfs_dest_path.display());
                fs::remove_dir_all(&nfs_dest_path)?;
            } else {
                println!("    Removing existing file on NFS: {}", nfs_dest_path.display());
                fs::remove_file(&nfs_dest_path)?;
            }
        }
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = nfs_dest_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Copy recursively
        if local_path.is_dir() {
            self.copy_directory_recursive(&local_path, &nfs_dest_path).await?;
        } else {
            // For files, copy to parent directory
            if let Some(parent) = nfs_dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&local_path, &nfs_dest_path)?;
        }
        
        // Fix permissions for Docker compatibility
        self.fix_permissions_recursive(&nfs_dest_path).await?;
        
        // Update the volume definition path to point to NFS
        volume_def.path = nfs_dest_path.to_string_lossy().to_string();
        
        println!("    Successfully copied to NFS: {}", nfs_dest_path.display());
        
        Ok(())
    }

    async fn copy_directory_recursive(&self, src: &Path, dst: &Path) -> Result<()> {
        if !src.is_dir() {
            return Err(anyhow::anyhow!("Source is not a directory: {}", src.display()));
        }
        
        fs::create_dir_all(dst)?;
        
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            
            if file_type.is_dir() {
                // Use Box::pin for recursive async call
                Box::pin(self.copy_directory_recursive(&src_path, &dst_path)).await?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }
        
        Ok(())
    }

    async fn fix_permissions_recursive(&self, path: &Path) -> Result<()> {
        println!("    Fixing permissions for Docker compatibility...");
        
        // Use chmod command to set appropriate permissions
        let output = Command::new("chmod")
            .args(&["-R", "755", path.to_str().unwrap()])
            .output()?;
        
        if output.status.success() {
            println!("    Successfully set directory permissions to 755");
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            println!("    Warning: Failed to set directory permissions: {}", error);
        }
        
        // For files, set 644 permissions (readable by all, writable by owner)
        let output = Command::new("find")
            .args(&[path.to_str().unwrap(), "-type", "f", "-exec", "chmod", "644", "{}", ";"])
            .output()?;
        
        if output.status.success() {
            println!("    Successfully set file permissions to 644");
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            println!("    Warning: Failed to set file permissions: {}", error);
        }
        
        // Change ownership to a more Docker-friendly user/group if possible
        // Try to use the current user or a common Docker user
        let current_user = std::env::var("SUDO_USER").ok()
            .or_else(|| std::env::var("USER").ok())
            .unwrap_or_else(|| "1000".to_string());
        
        let output = Command::new("chown")
            .args(&["-R", &format!("{}:{}", current_user, current_user), path.to_str().unwrap()])
            .output()?;
        
        if output.status.success() {
            println!("    Successfully changed ownership to {}", current_user);
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            println!("    Warning: Failed to change ownership: {}", error);
        }
        
        Ok(())
    }

    async fn process_compose_volumes(&self, compose_content: &str, volumes_definitions: &[VolumeDefinition]) -> Result<String> {
        println!("    Parsing docker-compose content...");
        
        // Parse the compose content to find volume references
        let mut yaml_value: serde_yaml::Value = serde_yaml::from_str(compose_content)?;
        println!("    Successfully parsed YAML content");
        
        // Process services section
        if let Some(services) = yaml_value.get_mut("services") {
            println!("    Found services section, processing {} services", 
                services.as_mapping().map(|m| m.len()).unwrap_or(0));
            
            if let Some(services_mapping) = services.as_mapping_mut() {
                for (service_name, service) in services_mapping {
                    let service_name_str = service_name.as_str().unwrap_or("unknown");
                    println!("    Processing service: {}", service_name_str);
                    
                    if let Some(volumes) = service.get_mut("volumes") {
                        println!("    Found volumes section in service {}", service_name_str);
                        self.process_service_volumes(volumes, volumes_definitions).await?;
                    } else {
                        println!("    No volumes section found in service {}", service_name_str);
                    }
                }
            }
        } else {
            println!("    No services section found in docker-compose");
        }
        
        // Convert back to string
        println!("    Converting modified YAML back to string...");
        let modified_content = serde_yaml::to_string(&yaml_value)?;
        println!("    Successfully converted YAML to string ({} characters)", modified_content.len());
        
        Ok(modified_content)
    }

    async fn process_service_volumes(&self, volumes: &mut serde_yaml::Value, volumes_definitions: &[VolumeDefinition]) -> Result<()> {
        println!("      Processing service volumes...");
        
        match volumes {
            serde_yaml::Value::Sequence(seq) => {
                println!("      Found {} volume entries", seq.len());
                
                for (index, volume) in seq.iter_mut().enumerate() {
                    println!("      Processing volume entry {}: {:?}", index, volume);
                    
                    if let Some(volume_str) = volume.as_str() {
                        println!("      Volume string: '{}'", volume_str);
                        
                        // Check if this is a volume reference (format: volume_id:container_path)
                        if volume_str.contains(':') {
                            let parts: Vec<&str> = volume_str.split(':').collect();
                            println!("      Split into {} parts: {:?}", parts.len(), parts);
                            
                            if parts.len() == 2 {
                                let volume_id = parts[0];
                                let container_path = parts[1];
                                println!("      Volume ID: '{}', Container path: '{}'", volume_id, container_path);
                                
                                // Find the volume definition
                                if let Some(volume_def) = volumes_definitions.iter().find(|v| v.id == volume_id) {
                                    println!("      Found volume definition: {:?}", volume_def);
                                    
                                    match volume_def.r#type {
                                        VolumeType::Volume => {
                                            // For Docker volumes, use the path as volume name
                                            let volume_path = format!("{}:{}", volume_def.path, container_path);
                                            println!("      Replacing Docker volume {} with: {}", volume_id, volume_path);
                                            *volume = serde_yaml::Value::String(volume_path);
                                        }
                                        VolumeType::Binding => {
                                            // For bindings, replace with NFS path
                                            // The path in volume_def.path is the NFS path after processing
                                            let nfs_path = format!("{}:{}", volume_def.path, container_path);
                                            println!("      Replacing binding volume {} with NFS path: {}", volume_id, nfs_path);
                                            *volume = serde_yaml::Value::String(nfs_path);
                                        }
                                    }
                                } else {
                                    println!("      Warning: Volume definition not found for ID: '{}'", volume_id);
                                    println!("      Available volume definitions: {:?}", 
                                        volumes_definitions.iter().map(|v| &v.id).collect::<Vec<_>>());
                                }
                            } else {
                                println!("      Volume string does not have exactly 2 parts, skipping");
                            }
                        } else {
                            println!("      Volume string does not contain ':', skipping");
                        }
                    } else {
                        println!("      Volume entry is not a string, skipping");
                    }
                }
            }
            _ => {
                println!("      Volume format is not a sequence, skipping");
            }
        }
        
        println!("      Finished processing service volumes");
        Ok(())
    }
} 