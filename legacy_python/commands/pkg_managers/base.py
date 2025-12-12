from abc import ABC, abstractmethod
from typing import List, Optional
from dataclasses import dataclass

@dataclass
class PackageResult:
    manager_name: str
    name: str
    version: str
    description: str
    installed: bool
    identifier: str # Often same as name, but sometimes id (winget)

    def __str__(self):
        status = "[installed]" if self.installed else ""
        return f"[{self.manager_name}] {self.name} ({self.version}) {status} - {self.description}"

class PackageManager(ABC):
    @property
    @abstractmethod
    def name(self) -> str:
        pass

    @property
    @abstractmethod
    def display_name(self) -> str:
        pass

    @abstractmethod
    def is_available(self) -> bool:
        pass

    @abstractmethod
    def search(self, query: str) -> List[PackageResult]:
        pass

    @abstractmethod
    def install(self, package: PackageResult) -> bool:
        pass
        
    @abstractmethod
    def uninstall(self, package: PackageResult) -> bool: # Optional mostly
        pass
