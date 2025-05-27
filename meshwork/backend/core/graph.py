import networkx as nx
import matplotlib.pyplot as plt
from networkx import DiGraph
from pydantic import BaseModel, Field

from core.task import Task, Status


class TaskGraph(BaseModel):
    """A directed graph representing tasks and their dependencies."""

    graph: DiGraph = Field(default_factory=DiGraph)
    name: str = Field(default_factory=str)

    class Config:
        arbitrary_types_allowed = True

    def add_task(self, task: Task):
        """Add a task to the graph."""
        self.graph.add_node(task.id, task=task)
        for d in task.depends_on:
            self.graph.add_edge(d, task.id)

        self.set_blocked_tasks()

    def get_task(self, task_id: str):
        """Get a task from the graph."""
        return self.graph.nodes[task_id]["task"]

    def get_all_tasks(self):
        """Get all tasks from the graph."""
        return [self.graph.nodes[node]["task"] for node in self.graph.nodes]

    def edit_task(self, task_id: str, new_task: Task):
        """Edit a task in the graph."""
        self.graph.nodes[task_id]["task"] = new_task
        self.set_blocked_tasks()

    def set_blocked_tasks(self):
        """
        Iterate over graph and set tasks as blocked if their dependencies are not DONE.
        """
        for node in self.graph.nodes:
            task = self.graph.nodes[node]["task"]
            if not all(self.get_task(dep).status == Status.DONE for dep in task.depends_on):
                task.status = Status.BLOCKED

    def delete_task(self, task_id: str):
        """Delete a task from the graph."""
        self.graph.remove_node(task_id)
        # iterate over all nodes in the graph
        for node in self.graph.nodes:
            task = self.graph.nodes[node]["task"]
            if task_id in task.depends_on:
                task.depends_on.remove(task_id)
        self.set_blocked_tasks()

    def visualize(self):
        pos = nx.kamada_kawai_layout(self.graph)
        nx.draw(self.graph, pos=pos, with_labels=True, arrows=True)
        plt.show()  # This line is crucial to display the figure


# Example usage
if __name__ == "__main__":
    TG = TaskGraph(name="MyTaskGraph")
    TG.add_task(
        Task(
            id="123", name="My subtask task", description="Blocking task", depends_on=[]
        )
    )
    TG.add_task(
        Task(
            id="321",
            name="My main task",
            description="Very important task",
            depends_on=["123"],
        )
    )
    TG.visualize()
