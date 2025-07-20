#!/usr/bin/env python3
"""
DockerOps Manager Script
Unified script for installing, updating, and uninstalling DockerOps
"""

import os
import sys
import json
import shutil
import argparse
import subprocess
import platform
from pathlib import Path
from urllib.request import urlopen, Request
from urllib.error import URLError
import zipfile
import tarfile

# Configuration
REPO_OWNER = "TomBedinoVT"
REPO_NAME = "DockerOps"
BINARY_NAME = "dockerops"
GITHUB_API_BASE = "https://api.github.com"

class DockerOpsManager:
    def __init__(self):
        self.install_dir = Path("/usr/local/bin")
        self.binary_path = self.install_dir / BINARY_NAME
        self.backup_path = self.install_dir / f"{BINARY_NAME}.backup"
        self.temp_dir = Path("/tmp/dockerops_install")
        
    def get_system_info(self):
        """Get system architecture and OS"""
        system = platform.system().lower()
        machine = platform.machine().lower()
        
        # Map common architectures
        arch_map = {
            'x86_64': 'x86_64',
            'amd64': 'x86_64',
            'aarch64': 'aarch64',
            'arm64': 'aarch64',
            'armv7l': 'armv7',
            'armv6l': 'armv6'
        }
        
        arch = arch_map.get(machine, machine)
        
        # Map OS names
        os_map = {
            'linux': 'linux',
            'darwin': 'macos',
            'windows': 'windows'
        }
        
        os_name = os_map.get(system, system)
        
        return os_name, arch
    
    def get_latest_release(self):
        """Get the latest release information from GitHub"""
        url = f"{GITHUB_API_BASE}/repos/{REPO_OWNER}/{REPO_NAME}/releases/latest"
        
        try:
            with urlopen(Request(url, headers={'User-Agent': 'DockerOps-Manager'})) as response:
                data = json.loads(response.read())
                return data
        except URLError as e:
            print(f"❌ Error fetching latest release: {e}")
            sys.exit(1)
    
    def get_specific_release(self, version):
        """Get a specific release by tag"""
        url = f"{GITHUB_API_BASE}/repos/{REPO_OWNER}/{REPO_NAME}/releases/tags/{version}"
        
        try:
            with urlopen(Request(url, headers={'User-Agent': 'DockerOps-Manager'})) as response:
                data = json.loads(response.read())
                return data
        except URLError as e:
            print(f"❌ Error fetching release {version}: {e}")
            sys.exit(1)
    
    def find_asset(self, release_data, os_name, arch):
        """Find the appropriate asset for the current system"""
        target_name = f"dockerops-{os_name}-{arch}"
        
        for asset in release_data.get('assets', []):
            asset_name = asset['name']
            if target_name in asset_name:
                return asset
        
        # Fallback: try to find any asset for the OS
        for asset in release_data.get('assets', []):
            asset_name = asset['name']
            if os_name in asset_name:
                return asset
        
        print(f"❌ No suitable asset found for {os_name}-{arch}")
        print("Available assets:")
        for asset in release_data.get('assets', []):
            print(f"  - {asset['name']}")
        sys.exit(1)
    
    def download_file(self, url, filename):
        """Download a file from URL"""
        print(f"📥 Downloading {filename}...")
        
        try:
            with urlopen(Request(url, headers={'User-Agent': 'DockerOps-Manager'})) as response:
                with open(filename, 'wb') as f:
                    shutil.copyfileobj(response, f)
            return True
        except URLError as e:
            print(f"❌ Download failed: {e}")
            return False
    
    def extract_archive(self, archive_path, extract_to):
        """Extract archive (zip or tar.gz)"""
        print(f"📦 Extracting {archive_path}...")
        
        try:
            # Convert Path to string for endswith check
            archive_str = str(archive_path)
            
            if archive_str.endswith('.zip'):
                with zipfile.ZipFile(archive_path, 'r') as zip_ref:
                    zip_ref.extractall(extract_to)
            elif archive_str.endswith('.tar.gz'):
                with tarfile.open(archive_path, 'r:gz') as tar_ref:
                    tar_ref.extractall(extract_to)
            else:
                print(f"❌ Unsupported archive format: {archive_path}")
                return False
            return True
        except Exception as e:
            print(f"❌ Extraction failed: {e}")
            return False
    
    def backup_existing(self):
        """Backup existing installation"""
        if self.binary_path.exists():
            print(f"💾 Backing up existing {BINARY_NAME}...")
            shutil.copy2(self.binary_path, self.backup_path)
            return True
        return False
    
    def restore_backup(self):
        """Restore from backup"""
        if self.backup_path.exists():
            print(f"🔄 Restoring from backup...")
            shutil.copy2(self.backup_path, self.binary_path)
            self.backup_path.unlink()
            return True
        return False
    
    def install_binary(self, binary_path):
        """Install the binary to the system"""
        print(f"🔧 Installing {BINARY_NAME} to {self.install_dir}...")
        
        try:
            # Ensure install directory exists
            self.install_dir.mkdir(parents=True, exist_ok=True)
            
            # Copy binary
            shutil.copy2(binary_path, self.binary_path)
            
            # Make executable
            self.binary_path.chmod(0o755)
            
            print(f"✅ {BINARY_NAME} installed successfully!")
            return True
        except Exception as e:
            print(f"❌ Installation failed: {e}")
            return False
    
    def get_user_home(self):
        """Get the home directory of the user who ran sudo"""
        sudo_user = os.environ.get('SUDO_USER')
        if sudo_user:
            return Path(f"/home/{sudo_user}")
        else:
            return Path.home()
    
    def clean_database(self):
        """Clean DockerOps database and directories"""
        print("🧹 Cleaning DockerOps database and directories...")
        
        home_dir = self.get_user_home()
        dockerops_dir = home_dir / ".dockerops"
        
        if dockerops_dir.exists():
            try:
                shutil.rmtree(dockerops_dir)
                print(f"✅ Removed {dockerops_dir}")
                return True
            except Exception as e:
                print(f"❌ Failed to remove {dockerops_dir}: {e}")
                return False
        else:
            print("ℹ️  No DockerOps directory found to clean")
            return True
    
    def clean_systemd(self):
        """Remove systemd service files if they exist"""
        systemd_dirs = [
            Path("/etc/systemd/system"),
            Path("/usr/lib/systemd/system"),
            Path("/lib/systemd/system")
        ]
        
        service_files = ["dockerops.service", "dockerops.timer"]
        
        for systemd_dir in systemd_dirs:
            if systemd_dir.exists():
                for service_file in service_files:
                    service_path = systemd_dir / service_file
                    if service_path.exists():
                        print(f"🗑️  Removing systemd service: {service_path}")
                        try:
                            service_path.unlink()
                            print(f"✅ Removed {service_path}")
                        except Exception as e:
                            print(f"⚠️  Failed to remove {service_path}: {e}")
        
        # Reload systemd if any services were removed
        try:
            subprocess.run(["systemctl", "daemon-reload"], check=True, capture_output=True)
            print("🔄 Systemd daemon reloaded")
        except subprocess.CalledProcessError:
            print("⚠️  Failed to reload systemd daemon")
    
    def clean_cron(self):
        """Remove cron jobs for DockerOps"""
        print("🔍 Checking for cron jobs...")
        
        try:
            # Check current user's crontab
            result = subprocess.run(["crontab", "-l"], capture_output=True, text=True)
            if result.returncode == 0:
                cron_content = result.stdout
                if "dockerops" in cron_content.lower():
                    print("⚠️  Found DockerOps entries in crontab")
                    print("   Please manually remove them with: crontab -e")
        except Exception:
            pass
    
    def clean_temp_files(self):
        """Clean temporary installation files"""
        if self.temp_dir.exists():
            try:
                shutil.rmtree(self.temp_dir)
                print(f"🧹 Cleaned temporary files: {self.temp_dir}")
            except Exception as e:
                print(f"⚠️  Failed to clean temp files: {e}")
    
    def get_current_version(self):
        """Get current installed version"""
        if self.binary_path.exists():
            try:
                result = subprocess.run([str(self.binary_path), "version"], 
                                      capture_output=True, text=True, timeout=10)
                if result.returncode == 0:
                    # Extract version from output
                    for line in result.stdout.split('\n'):
                        if 'DockerOps CLI v' in line:
                            return line.split('v')[1].strip()
            except Exception:
                pass
        return None
    
    def remove_binary(self):
        """Remove the DockerOps binary"""
        if self.binary_path.exists():
            print(f"🗑️  Removing {self.binary_path}...")
            try:
                self.binary_path.unlink()
                print("✅ Binary removed successfully")
                return True
            except Exception as e:
                print(f"❌ Failed to remove binary: {e}")
                return False
        else:
            print("ℹ️  No binary found to remove")
            return True
    
    def remove_backup(self):
        """Remove backup file if it exists"""
        if self.backup_path.exists():
            print(f"🗑️  Removing backup file {self.backup_path}...")
            try:
                self.backup_path.unlink()
                print("✅ Backup removed")
                return True
            except Exception as e:
                print(f"⚠️  Failed to remove backup: {e}")
                return False
        return True
    
    def install(self, version=None, clean_db=False, clean_dirs=False):
        """Main installation method"""
        print("🚀 DockerOps Installer")
        print("=" * 50)
        
        # Check if running as root
        if os.geteuid() != 0:
            print("❌ This script must be run as root (use sudo)")
            sys.exit(1)
        
        # Get system info
        os_name, arch = self.get_system_info()
        print(f"🖥️  System: {os_name}-{arch}")
        
        # Get current version
        current_version = self.get_current_version()
        if current_version:
            print(f"📋 Current version: {current_version}")
        
        # Get release data
        if version:
            print(f"🎯 Installing specific version: {version}")
            release_data = self.get_specific_release(version)
        else:
            print("🔄 Installing latest version")
            release_data = self.get_latest_release()
        
        release_version = release_data['tag_name']
        print(f"📦 Release version: {release_version}")
        
        # Check if already installed
        if current_version == release_version:
            print("ℹ️  Already running the latest version")
            if not clean_db and not clean_dirs:
                return
        
        # Find appropriate asset
        asset = self.find_asset(release_data, os_name, arch)
        print(f"📦 Asset: {asset['name']}")
        
        # Create temp directory
        self.temp_dir.mkdir(exist_ok=True)
        
        # Download file
        download_path = self.temp_dir / asset['name']
        if not self.download_file(asset['browser_download_url'], download_path):
            self.clean_temp_files()
            sys.exit(1)
        
        # Extract archive
        if not self.extract_archive(download_path, self.temp_dir):
            self.clean_temp_files()
            sys.exit(1)
        
        # Find the binary in extracted files
        binary_found = None
        for file_path in self.temp_dir.rglob("*"):
            if file_path.is_file() and file_path.name == BINARY_NAME:
                binary_found = file_path
                break
        
        if not binary_found:
            print(f"❌ Could not find {BINARY_NAME} in extracted files")
            self.clean_temp_files()
            sys.exit(1)
        
        # Backup existing installation
        self.backup_existing()
        
        # Install new binary
        if not self.install_binary(binary_found):
            print("❌ Installation failed, restoring backup...")
            self.restore_backup()
            self.clean_temp_files()
            sys.exit(1)
        
        # Clean database and directories if requested
        if clean_db or clean_dirs:
            self.clean_database()
        
        # Clean temp files
        self.clean_temp_files()
        
        # Verify installation
        new_version = self.get_current_version()
        if new_version:
            print(f"✅ Installation complete! New version: {new_version}")
        else:
            print("✅ Installation complete!")
        
        print(f"🎉 {BINARY_NAME} is now available at: {self.binary_path}")
    
    def uninstall(self, clean_all=False):
        """Main uninstallation method"""
        print("🗑️  DockerOps Uninstaller")
        print("=" * 50)
        
        # Check if running as root
        if os.geteuid() != 0:
            print("❌ This script must be run as root (use sudo)")
            sys.exit(1)
        
        # Get current version before removal
        current_version = self.get_current_version()
        if current_version:
            print(f"📋 Current version: {current_version}")
        
        # Remove binary
        if not self.remove_binary():
            print("❌ Failed to remove binary")
            sys.exit(1)
        
        # Remove backup
        self.remove_backup()
        
        # Clean database and directories
        if clean_all:
            if not self.clean_database():
                print("❌ Failed to clean database")
                sys.exit(1)
            
            # Clean systemd services
            self.clean_systemd()
            
            # Check cron jobs
            self.clean_cron()
        
        print("✅ Uninstallation complete!")
        
        if clean_all:
            print("🧹 All DockerOps files and data have been removed")
        else:
            print("ℹ️  Binary removed. Use --clean-all to remove all data")
    
    def status(self):
        """Show current status"""
        print("📊 DockerOps Status")
        print("=" * 50)
        
        # Check if binary exists
        if self.binary_path.exists():
            print(f"✅ Binary found: {self.binary_path}")
            
            # Get version
            version = self.get_current_version()
            if version:
                print(f"📋 Version: {version}")
            else:
                print("⚠️  Could not determine version")
            
            # Check permissions
            stat = self.binary_path.stat()
            if stat.st_mode & 0o111:  # Check if executable
                print("✅ Binary is executable")
            else:
                print("❌ Binary is not executable")
        else:
            print("❌ Binary not found")
        
        # Check database
        home_dir = self.get_user_home()
        dockerops_dir = home_dir / ".dockerops"
        if dockerops_dir.exists():
            print(f"📁 Data directory: {dockerops_dir}")
            db_file = dockerops_dir / "dockerops.db"
            if db_file.exists():
                print(f"💾 Database file: {db_file}")
                size = db_file.stat().st_size
                print(f"📏 Database size: {size} bytes")
            else:
                print("⚠️  Database file not found")
        else:
            print("ℹ️  No data directory found")
        
        # Check systemd services
        systemd_dirs = [
            Path("/etc/systemd/system"),
            Path("/usr/lib/systemd/system"),
            Path("/lib/systemd/system")
        ]
        
        service_files = ["dockerops.service", "dockerops.timer"]
        services_found = []
        
        for systemd_dir in systemd_dirs:
            if systemd_dir.exists():
                for service_file in service_files:
                    service_path = systemd_dir / service_file
                    if service_path.exists():
                        services_found.append(service_path)
        
        if services_found:
            print("🔧 Systemd services found:")
            for service in services_found:
                print(f"   - {service}")
        else:
            print("ℹ️  No systemd services found")

def main():
    parser = argparse.ArgumentParser(
        description="DockerOps Manager - Install, update, and uninstall DockerOps",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Commands:
  install                 Install or update DockerOps
  uninstall              Uninstall DockerOps
  status                 Show current status

Examples:
  %(prog)s install                    # Install latest version
  %(prog)s install -v v1.0.0         # Install specific version
  %(prog)s install --clean-all       # Install and clean everything
  %(prog)s uninstall --clean-all     # Uninstall and clean everything
  %(prog)s status                    # Show current status
        """
    )
    
    subparsers = parser.add_subparsers(dest='command', help='Available commands')
    
    # Install command
    install_parser = subparsers.add_parser('install', help='Install or update DockerOps')
    install_parser.add_argument(
        '-v', '--version',
        help='Install specific version (e.g., v1.0.0)'
    )
    install_parser.add_argument(
        '--clean-db',
        action='store_true',
        help='Clean DockerOps database after installation'
    )
    install_parser.add_argument(
        '--clean-dirs',
        action='store_true',
        help='Clean DockerOps directories after installation'
    )
    install_parser.add_argument(
        '--clean-all',
        action='store_true',
        help='Clean database and directories after installation'
    )
    
    # Uninstall command
    uninstall_parser = subparsers.add_parser('uninstall', help='Uninstall DockerOps')
    uninstall_parser.add_argument(
        '--clean-all',
        action='store_true',
        help='Remove binary, database, and all related files'
    )
    
    # Status command
    subparsers.add_parser('status', help='Show current DockerOps status')
    
    args = parser.parse_args()
    
    if not args.command:
        parser.print_help()
        sys.exit(1)
    
    # Create manager and run command
    manager = DockerOpsManager()
    
    if args.command == 'install':
        # Handle clean-all flag
        clean_db = args.clean_db or args.clean_all
        clean_dirs = args.clean_dirs or args.clean_all
        
        manager.install(
            version=args.version,
            clean_db=clean_db,
            clean_dirs=clean_dirs
        )
    elif args.command == 'uninstall':
        manager.uninstall(clean_all=args.clean_all)
    elif args.command == 'status':
        manager.status()

if __name__ == "__main__":
    main() 