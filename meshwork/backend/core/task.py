import uuid
from enum import Enum

from pydantic import BaseModel, Field


class Status(Enum):
    TODO = 0
    IN_PROGRESS = 1
    DONE = 2
    REVIEW = 3
    BLOCKED = 4


class CompletionCondition(BaseModel):
    id: str = Field(default=uuid.uuid4().hex)


class Task(BaseModel):
    id: str = Field(default=uuid.uuid4().hex)
    """Task identificator"""
    name: str = Field(default_factory=str)
    """Task name"""
    description: str = Field(default_factory=str)
    """Task description"""
    depends_on: list[str] = Field(default_factory=list)
    """List of other tasks' IDs that must be completed before this one"""
    users: list[str] = Field(default_factory=list)
    """List of users' IDs who are responsible for this task"""
    tags: list[str] = Field(default_factory=list)
    """Task tags"""
    status: Status = Field(default=Status.TODO)
    """Task status"""
    # completion_condition: Optional[CompletionCondition]
    # """Condition that need to be met for the task to be completed"""
