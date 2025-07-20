#!/bin/bash

# DockerOps Manager Script
# Simple bash wrapper for the Python manager

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO_OWNER="TomBedinoVT"
REPO_NAME="DockerOps"
MANAGER_SCRIPT="dockerops_manager.py"

show_help() {
    echo -e "${BLUE}🚀 DockerOps Manager${NC}"
    echo "======================"
    echo ""
    echo "Usage: $0 <command> [options]"
    echo ""
    echo "Commands:"
    echo "  install                 Install or update DockerOps"
    echo "  uninstall              Uninstall DockerOps"
    echo "  status                 Show current status"
    echo "  help                   Show this help message"
    echo ""
    echo "Install options:"
    echo "  -v, --version VERSION  Install specific version (e.g., v1.0.0)"
    echo "  --clean-db            Clean database after installation"
    echo "  --clean-dirs          Clean directories after installation"
    echo "  --clean-all           Clean everything after installation"
    echo ""
    echo "Uninstall options:"
    echo "  --clean-all           Remove binary and all data"
    echo ""
    echo "Examples:"
    echo "  $0 install                    # Install latest version"
    echo "  $0 install -v v1.0.0         # Install specific version"
    echo "  $0 install --clean-all       # Install and clean everything"
    echo "  $0 uninstall --clean-all     # Uninstall and clean everything"
    echo "  $0 status                    # Show current status"
    echo ""
    echo "Quick install (one-liner):"
    echo "  curl -sSL https://raw.githubusercontent.com/$REPO_OWNER/$REPO_NAME/main/dockerops.sh | sudo bash -s install"
}

download_manager() {
    echo -e "${BLUE}📥 Downloading DockerOps Manager...${NC}"
    if curl -s -o "$MANAGER_SCRIPT" "https://raw.githubusercontent.com/$REPO_OWNER/$REPO_NAME/main/$MANAGER_SCRIPT"; then
        echo -e "${GREEN}✅ Download successful${NC}"
        chmod +x "$MANAGER_SCRIPT"
        return 0
    else
        echo -e "${RED}❌ Failed to download manager script${NC}"
        return 1
    fi
}

check_prerequisites() {
    # Check if running as root for install/uninstall commands
    if [[ "$1" == "install" || "$1" == "uninstall" ]]; then
        if [[ $EUID -ne 0 ]]; then
            echo -e "${RED}❌ This command must be run as root (use sudo)${NC}"
            exit 1
        fi
    fi
    
    # Check if Python 3 is available
    if ! command -v python3 &> /dev/null; then
        echo -e "${RED}❌ Python 3 is required but not installed${NC}"
        echo "Please install Python 3 and try again"
        exit 1
    fi
    
    # Check if curl is available
    if ! command -v curl &> /dev/null; then
        echo -e "${RED}❌ curl is required but not installed${NC}"
        echo "Please install curl and try again"
        exit 1
    fi
}

main() {
    # Show help if no arguments
    if [[ $# -eq 0 ]]; then
        show_help
        exit 0
    fi
    
    # Handle help command
    if [[ "$1" == "help" || "$1" == "-h" || "$1" == "--help" ]]; then
        show_help
        exit 0
    fi
    
    # Check prerequisites
    check_prerequisites "$1"
    
    # Download manager script if it doesn't exist
    if [[ ! -f "$MANAGER_SCRIPT" ]]; then
        if ! download_manager; then
            exit 1
        fi
    fi
    
    # Run the manager script
    echo -e "${BLUE}🔧 Running DockerOps Manager...${NC}"
    if python3 "$MANAGER_SCRIPT" "$@"; then
        echo -e "${GREEN}✅ Command completed successfully!${NC}"
        
        # Clean up manager script if it was downloaded
        if [[ -f "$MANAGER_SCRIPT" ]]; then
            rm -f "$MANAGER_SCRIPT"
            echo -e "${BLUE}🧹 Cleaned up manager script${NC}"
        fi
    else
        echo -e "${RED}❌ Command failed${NC}"
        echo -e "${YELLOW}💡 The manager script is still available as: $MANAGER_SCRIPT${NC}"
        exit 1
    fi
}

# Run main function with all arguments
main "$@" 