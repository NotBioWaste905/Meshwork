from pydantic import BaseModel, Field

class User(BaseModel):
    id: int = Field(..., title="User ID")
    email: str = Field(..., title="Email Address")
    name: str = Field(..., title="Full Name")
    graphs: list[str] = Field(..., title="Graph IDs")
