import {useState} from "preact/hooks";
import {
    StatusCodes,
} from "https://deno.land/x/https_status_codes/mod.ts";
interface RegisterDetails {
    username: string;
    email: string;
    password: string;
}

export const SignUp = () => {
    const [status, setStatus] = useState<"succeeded" | "failed">();
    const [state, setState] = useState<RegisterDetails>({
        username: "",
        email: "",
        password: "",
    });

    const handleChange = (e: Event) => {
        const target = e.target as HTMLInputElement;
        const name = target.name;
        const value = target.value;
        setState({...state, [name]: value});
    }

    const sendForm = async (e: Event) => {
        e.preventDefault();
        try {
            const response = await fetch("http://127.0.0.1:8000/api/auth/register", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json"
                },
                body: JSON.stringify({
                    username: state.username,
                    email: state.email,
                    password: state.password
                })
            })
            if (response.status !== StatusCodes.CREATED) {
                setStatus("failed");
                return;
            }
            setStatus("succeeded");
        } catch (e) {
            setStatus("failed");
            console.error(e);
        }
    }
    //サインアップ画面を作成する
    return (
        <div className="flex flex-col items-center justify-center min-h-screen bg-gray-100">
            <div className="bg-white p-6 rounded-lg shadow-lg w-80">
                <h2 className="text-2xl font-bold mb-5 text-gray-800">Create Your Account</h2>
                <form className="space-y-5" onSubmit={sendForm}>
                    <div>
                        <label className="block text-sm font-medium text-gray-700">User Name</label>
                        <input type="text"
                               name="username"
                               value={state.username}
                               onChange={handleChange}
                               className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50"
                        />
                    </div>
                    <div>
                        <label className="block text-sm font-medium text-gray-700">Email address</label>
                        <input type="email"
                               name="email"
                               value={state.email}
                               onChange={handleChange}
                               className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50"/>
                    </div>
                    <div>
                        <label className="block text-sm font-medium text-gray-700">Password</label>
                        <input type="password"
                               name="password"
                               value={state.password}
                               onChange={handleChange}
                               className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50"/>
                    </div>
                    <div>
                        <button type="submit"
                                className="w-full py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500">
                            Sign up
                        </button>
                    </div>
                </form>
            </div>
        </div>
    )
}

export default SignUp;