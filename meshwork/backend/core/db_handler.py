import os
import logging
from typing import Optional, List, Dict, Any
from contextlib import contextmanager

from dotenv import load_dotenv
from neo4j import GraphDatabase, Driver, Session
from core.task import Task, Status
from core.user import User

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class Neo4jHandler:
    def __init__(self) -> None:
        load_dotenv()
        self.uri = os.getenv("NEO4J_URI")
        self.auth = (os.getenv("NEO4J_USERNAME"), os.getenv("NEO4J_PASSWORD"))

    def connect(self):
        self.driver = GraphDatabase.driver(self.uri, auth=self.auth)
        self.driver.verify_connectivity()
        logger.info("Connected to Neo4j backend.")

    def close(self):
        if self.driver:
            self.driver.close()
            logger.info("Disconnected from Neo4j backend.")

    def create_example(self):
        pass

    @contextmanager
    def session(self):
        with self.driver.session() as session:
            yield session

    def create_user(self, user: User) -> str:
        with self.session() as session:
            try:
                result = session.run(
                    """
                CREATE (u:User {id: $id, name: $name, email: $email, created_at: datetime()}) RETURN u.id
                """,
                    id=user.id,
                    name=user.name,
                    email=user.email,
                )
                return result.single()["u.id"]
            except Exception as e:
                logger.error(f"Failed to create user: {e}")
                raise

    def create_task_graph(self, user_id: str, graph_name: str) -> str:
        with self.session() as session:
            try:
                result = session.run(
                    """
                    MATCH (u:User {id: $user_id})
                    CREATE (tg:TaskGraph {id: randomUUID(), name: $graph_name, created_at: datetime()})
                    CREATE (u)-[:OWNS]->(tg)
                    CREATE (u)-[:PART_OF]->(tg)
                    RETURN tg.id
                    """,
                    user_id=user_id,
                    graph_name=graph_name,
                )
                return result.single()["tg.id"]
            except Exception as e:
                logger.error(f"Failed to create task graph: {e}")
                raise

    def get_user_graphs(self, user_id: str) -> List[Dict[str, Any]]:
        with self.session() as session:
            try:
                result = session.run(
                    """
                    MATCH (u:User {id: $user_id})-[:PART_OF]->(tg:TaskGraph)
                    RETURN tg.id as id, tg.name as name, tg.created_at as created_at
                    """,
                    user_id=user_id,
                )
                return [record.data() for record in result]
            except Exception as e:
                logger.error(f"Failed to get user graphs: {e}")
                raise
