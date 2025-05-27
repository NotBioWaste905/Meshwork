from fastapi import FastAPI

from core.task import Task
from core.graph import TaskGraph

app = FastAPI()
TG = TaskGraph()

@app.get("/")
async def root():
    return {"message": "Hello World"}


@app.post("/v0/task/add/")
async def add_task(task: Task):
    TG.add_task(task)
    return {"message": f"Task {task.id} added"}

@app.get("/v0/task/{task_id}")
async def get_task(task_id: str):
    return TG.get_task(task_id)

@app.get("/v0/task/get_all")
async def get_all_tasks():
    return TG.get_all_tasks()
