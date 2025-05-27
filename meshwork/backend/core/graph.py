from core.task import Task
from networkx import DiGraph
from pydantic import BaseModel, Field

class TaskGraph(BaseModel):
    graph: DiGraph
    name: str = Field(default_factory=str)

    async def add_task(self, task: Task):
        self.graph.add_node(task)
        for d in task.depends_on:
            self.graph.add_edge(
                self.graph.nodes()
            )
