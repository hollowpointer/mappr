#!/bin/bash
set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=======================================${NC}"
echo -e "${BLUE}   Mappr Integration Test Runner       ${NC}"
echo -e "${BLUE}=======================================${NC}"

# Check for correct directory or move to root
if [ -f "Cargo.toml" ]; then
    : # All good
elif [ -f "../Cargo.toml" ]; then
    cd ..
else
    echo -e "${RED}Error: Please run this script from the project root or tests/ directory.${NC}"
    exit 1
fi

# Check if running as root
if [ "$EUID" -ne 0 ]; then
  echo -e "${YELLOW}Note: Running without root privileges.${NC}"
  echo -e "${YELLOW}      Privileged tests (like Netns/ARP) will be SKIPPED.${NC}"
  
  echo -ne "${BLUE}Would you like to run with sudo to enable ALL tests? [Y/n]: ${NC}"
  read choice
  choice=${choice:-Y} # Default to Yes

  if [[ "$choice" =~ ^[Yy]$ ]]; then
      echo -e "${BLUE}Relaunching with sudo...${NC}"
      # -E preserves environment variables (PATH, Cargo stuff)
      exec sudo -E "$0" "$@"
  else
      echo -e "${YELLOW}Proceeding with limited test suite...${NC}"
  fi
else
  echo -e "${GREEN}Running with root privileges. Full test suite enabled.${NC}"
fi

echo ""
echo -e "${BLUE}Running cargo nextest for mappr-integration-tests...${NC}"
echo -e "${BLUE}---------------------------------------${NC}"

# Run the tests
cargo nextest run -p mappr-integration-tests

EXIT_CODE=$?

echo ""
if [ $EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}SUCCESS: All integration tests passed!${NC}"
else
    echo -e "${RED}FAILURE: Some tests failed.${NC}"
fi

exit $EXIT_CODE
