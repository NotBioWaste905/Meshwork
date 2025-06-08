import json
import datetime
import logging
import time
import uuid

import schedule
import networkx as nx
from networkx.readwrite import json_graph
from networkx import DiGraph
from pydantic import BaseModel, Field

from core.db_handler import Neo4jHandler
from core.task import Task, Status

logger = logging.getLogger(__name__)


class TaskGraph(BaseModel):
    """A directed graph representing tasks and their dependencies."""

    id: str = Field(default=uuid.uuid4().hex)
    graph: DiGraph = Field(default_factory=DiGraph)
    name: str = Field(default_factory=str)
    db_handler: Neo4jHandler = Field(exclude=True)

    class Config:
        arbitrary_types_allowed = True

    def __init__(self, **data):
        super().__init__(**data)
        if not hasattr(self, "db_handler"):
            self.db_handler = Neo4jHandler()

    def add_task(self, task: Task):
        """Add a task to the graph."""
        with self.db_handler.session() as session:
            session.run(
                """
                MATCH (tg:TaskGraph {id: $graph_id})
                CREATE (t:Task {id: $task_id,
                                    name: $name,
                                    description: $description,
                                    status: $status,
                                    tags: $tags,
                                    users: $users,
                                    created_at: datetime()})
                CREATE (tg)-[:CONTAINS]->(t)
                """,
                graph_id=self.id,
                task_id=task.id,
                name=task.name,
                description=task.description,
                status=task.status,
                tags=task.tags,
                users=task.users,
            )

            for dependency in task.depends_on:
                session.run(
                    """
                            MATCH (t1:Task {id: $task_id}), (t2:Task {id: $dep_id})
                            CREATE (t1)-[:DEPENDS_ON]->(t2)
                        """,
                    task_id=task.id,
                    dep_id=dependency,
                )

        self.set_blocked_tasks()

    def get_task(self, task_id: str):
        """Get a task from the graph."""
        return self.graph.nodes[task_id]["task"]

    def get_all_tasks(self):
        """Get all tasks from the graph."""
        if not self.graph.nodes:
            return []
        return [self.graph.nodes[node]["task"] for node in self.graph.nodes]

    def edit_task(self, task_id: str, task: Task):
        # TODO: take only the necessary fields from new_task
        """Edit a task in the graph."""
        self.graph.nodes[task_id]["task"] = task
        self.set_blocked_tasks()

    def connect_nodes(self, node_dependency: str, node_dependee: str):
        """Connect two nodes in the graph."""
        self.graph.add_edge(node_dependency, node_dependee)
        self.graph.nodes[node_dependee]["task"].depends_on.append(node_dependency)
        logger.info(f"Connected nodes {node_dependency} and {node_dependee}")

        self.set_blocked_tasks()

    def disconnect_nodes(self, node_dependency: str, node_dependee: str):
        """Disconnect two nodes in the graph."""
        self.graph.remove_edge(node_dependency, node_dependee)
        self.graph.nodes[node_dependee]["task"].depends_on.remove(node_dependency)
        logger.info(f"Disconnected nodes {node_dependency} and {node_dependee}")

        self.set_blocked_tasks()

    def set_blocked_tasks(self):
        """
        Iterate over graph and set tasks as blocked if their dependencies are not DONE.
        """
        # for node in self.graph.nodes:
        #     task = self.graph.nodes[node]["task"]
        #     if not all(
        #         self.get_task(dep).status == Status.DONE for dep in task.depends_on
        #     ):
        #         task.status = Status.BLOCKED
        #     else:
        #         if task.status == Status.BLOCKED:
        #             task.status = Status.TODO
        with self.db_handler.session() as session:
            pass

    def delete_task(self, task_id: str):
        """Delete a task from the graph."""
        self.graph.remove_node(task_id)
        # iterate over all nodes in the graph
        for node in self.graph.nodes:
            task = self.graph.nodes[node]["task"]
            if task_id in task.depends_on:
                task.depends_on.remove(task_id)
        self.set_blocked_tasks()


# Example usage
if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)  # <-- Enable logging
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

    while True:
        schedule.run_pending()
        time.sleep(1)
